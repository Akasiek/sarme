use std::collections::HashMap;
use std::path::Path;

use super::discovery::discover;
use super::error::ScanError;
use super::file::{DiscoveredFile, Discovery};
use super::runner::Scanner;
use super::summary::ScanSummary;
use crate::database::scans::{ScanProblem, ScanRepository, ScannedTrack, StoredTrack, TrackChange};
use crate::metadata::{MetadataIssue, MetadataRead};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tracing::error;

pub(super) async fn scan(scanner: &Scanner) -> Result<ScanSummary, ScanError> {
    let repository = scanner.repository();
    let scan_id = repository.create_run().await?;
    let result = run_scan(scanner.library_root(), repository, scan_id).await;

    if let Err(scan_error) = &result
        && let Err(persistence_error) = repository.fail(scan_id, &scan_error.to_string()).await
    {
        error!(
            scan_id,
            error = %persistence_error,
            "Could not mark failed library scan"
        );
    }

    result
}

async fn run_scan(
    library_root: &Path,
    repository: &ScanRepository,
    scan_id: i64,
) -> Result<ScanSummary, ScanError> {
    let root = library_root.to_path_buf();
    let discovery = tokio::task::spawn_blocking(move || discover(&root)).await??;
    let stored_tracks = repository.load_tracks().await?;
    let prepared =
        tokio::task::spawn_blocking(move || prepare_scan(scan_id, &discovery, stored_tracks))
            .await?;

    repository
        .complete(
            &prepared.tracks,
            &prepared.missing_track_ids,
            &prepared.problems,
            prepared.summary,
        )
        .await?;

    Ok(prepared.summary)
}

struct PreparedScan {
    tracks: Vec<ScannedTrack>,
    missing_track_ids: Vec<i64>,
    problems: Vec<ScanProblem>,
    summary: ScanSummary,
}

struct PreparedTrack {
    track: ScannedTrack,
    problems: Vec<ScanProblem>,
}

fn prepare_scan(
    scan_id: i64,
    discovery: &Discovery,
    stored_tracks: Vec<StoredTrack>,
) -> PreparedScan {
    let mut stored_by_path: HashMap<_, _> = stored_tracks
        .into_iter()
        .map(|track| (track.path.clone(), track))
        .collect();
    let mut tracks = Vec::with_capacity(discovery.files().len());
    let mut problems = discovery
        .issues()
        .iter()
        .map(|issue| ScanProblem {
            path: issue_path(discovery.root(), issue.path()),
            message: issue.message().to_owned(),
        })
        .collect::<Vec<_>>();
    let mut summary = ScanSummary::new(scan_id, usize_to_i64(discovery.files().len()));

    for file in discovery.files() {
        match prepare_track(discovery.root(), file, &mut stored_by_path) {
            Ok(prepared) => {
                match prepared.track.change {
                    TrackChange::New => summary.record_new(),
                    TrackChange::Changed => summary.record_updated(),
                    TrackChange::Unchanged => summary.record_unchanged(),
                }

                problems.extend(prepared.problems);
                tracks.push(prepared.track);
            }
            Err(problem) => problems.push(problem),
        }
    }

    // An incomplete traversal must not mark unseen files as removed.
    let missing_track_ids = if problems.is_empty() {
        stored_by_path
            .into_values()
            .filter(|track| track.missing_since.is_none())
            .map(|track| track.id)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    summary.finish(
        usize_to_i64(missing_track_ids.len()),
        usize_to_i64(problems.len()),
    );

    PreparedScan {
        tracks,
        missing_track_ids,
        problems,
        summary,
    }
}

fn prepare_track(
    root: &Path,
    file: &DiscoveredFile,
    stored_by_path: &mut HashMap<String, StoredTrack>,
) -> Result<PreparedTrack, ScanProblem> {
    let path = file.relative_path().to_str().ok_or_else(|| ScanProblem {
        path: file.relative_path().to_string_lossy().into_owned(),
        message: "audio file path contains non-UTF-8 data".to_owned(),
    })?;
    let file_size = i64::try_from(file.file_size()).map_err(|_| ScanProblem {
        path: path.to_owned(),
        message: "audio file size exceeds SQLite integer range".to_owned(),
    })?;
    let modified_at = format_time(file.modified_at()).map_err(|message| ScanProblem {
        path: path.to_owned(),
        message,
    })?;
    let (lrc_path, lrc_problem) = match relative_lrc_path(root, file) {
        Ok(lrc_path) => (lrc_path, None),
        Err(problem) => (None, Some(problem)),
    };
    let stored = stored_by_path.remove(path);
    let change = classify(stored.as_ref(), file_size, &modified_at);
    let mut problems = lrc_problem.into_iter().collect::<Vec<_>>();
    let metadata = if change == TrackChange::Unchanged {
        None
    } else {
        Some(match crate::metadata::read(file.absolute_path()) {
            Ok(metadata) => MetadataRead::Complete(metadata),
            Err(error) => {
                let message = error.to_string();
                problems.push(ScanProblem {
                    path: path.to_owned(),
                    message: message.clone(),
                });
                MetadataRead::Failed(MetadataIssue::read_error(message))
            }
        })
    };

    Ok(PreparedTrack {
        track: ScannedTrack {
            id: stored.map(|track| track.id),
            path: path.to_owned(),
            file_size,
            modified_at,
            lrc_path,
            change,
            metadata,
        },
        problems,
    })
}

fn classify(stored: Option<&StoredTrack>, file_size: i64, modified_at: &str) -> TrackChange {
    let Some(stored) = stored else {
        return TrackChange::New;
    };

    if stored.file_size == file_size
        && stored.modified_at == modified_at
        && stored.missing_since.is_none()
    {
        TrackChange::Unchanged
    } else {
        TrackChange::Changed
    }
}

fn relative_lrc_path(root: &Path, file: &DiscoveredFile) -> Result<Option<String>, ScanProblem> {
    let Some(lrc_path) = file.lrc_path() else {
        return Ok(None);
    };
    let relative_path = lrc_path.strip_prefix(root).map_err(|_| ScanProblem {
        path: lrc_path.to_string_lossy().into_owned(),
        message: "LRC file is outside the canonical music library root".to_owned(),
    })?;
    let relative_path = relative_path.to_str().ok_or_else(|| ScanProblem {
        path: relative_path.to_string_lossy().into_owned(),
        message: "LRC file path contains non-UTF-8 data".to_owned(),
    })?;

    Ok(Some(relative_path.to_owned()))
}

fn format_time(system_time: std::time::SystemTime) -> Result<String, String> {
    OffsetDateTime::from(system_time)
        .format(&Rfc3339)
        .map_err(|error| format!("could not format file modification time: {error}"))
}

fn issue_path(root: &Path, path: Option<&Path>) -> String {
    path.map_or_else(
        || root.to_string_lossy().into_owned(),
        |path| {
            path.strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned()
        },
    )
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
