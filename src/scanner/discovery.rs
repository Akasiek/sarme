use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use super::error::{DiscoveryError, ScanIssue};
use super::file::{DiscoveredFile, Discovery};

const AUDIO_EXTENSIONS: [&str; 5] = ["flac", "mp3", "opus", "ogg", "m4a"];

pub(crate) fn discover(library_root: &Path) -> Result<Discovery, DiscoveryError> {
    let root =
        library_root
            .canonicalize()
            .map_err(|source| DiscoveryError::LibraryUnavailable {
                path: library_root.to_path_buf(),
                source,
            })?;
    let metadata = root
        .metadata()
        .map_err(|source| DiscoveryError::LibraryUnavailable {
            path: root.clone(),
            source,
        })?;

    if !metadata.is_dir() {
        return Err(DiscoveryError::LibraryNotDirectory { path: root });
    }

    let mut discovery = Discovery::new(root.clone());
    let walker = WalkDir::new(&root)
        .min_depth(1)
        .follow_links(false)
        .follow_root_links(false)
        .into_iter();

    for entry in walker {
        match entry {
            Ok(entry) => process_file_entry(&root, &mut discovery, entry),
            Err(error) => {
                discovery.add_issue(ScanIssue::from_walkdir(&error));
            }
        }
    }

    discovery.sort_files();

    Ok(discovery)
}

fn process_file_entry(root: &Path, discovery: &mut Discovery, entry: DirEntry) {
    if entry.path_is_symlink() || !entry.file_type().is_file() {
        return;
    }

    let path = entry.into_path();
    if !is_supported_audio_file(&path) {
        return;
    }

    let Ok(relative_path) = path.strip_prefix(root) else {
        discovery.add_issue(ScanIssue::invalid_path(&path));
        return;
    };
    let relative_path = relative_path.to_path_buf();
    let metadata = match path.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            discovery.add_issue(ScanIssue::from_io(&path, &error));
            return;
        }
    };
    let modified_at = match metadata.modified() {
        Ok(modified_at) => modified_at,
        Err(error) => {
            discovery.add_issue(ScanIssue::from_io(&path, &error));
            return;
        }
    };
    let lrc_path = detect_lrc(&path, discovery);

    discovery.add_file(DiscoveredFile::new(
        path,
        relative_path,
        metadata.len(),
        modified_at,
        lrc_path,
    ));
}

fn is_supported_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

fn detect_lrc(path: &Path, discovery: &mut Discovery) -> Option<PathBuf> {
    let lrc_path = path.with_extension("lrc");

    match lrc_path.symlink_metadata() {
        Ok(_) => Some(lrc_path),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            discovery.add_issue(ScanIssue::from_io(&lrc_path, &error));
            None
        }
    }
}
