use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::metadata::{MetadataRead, TrackMetadata};
use crate::scanner::summary::ScanSummary;

#[derive(Debug)]
pub(crate) struct StoredTrack {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) file_size: i64,
    pub(crate) modified_at: String,
    pub(crate) missing_since: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrackChange {
    New,
    Changed,
    Unchanged,
}

#[derive(Debug)]
pub(crate) struct ScannedTrack {
    pub(crate) id: Option<i64>,
    pub(crate) path: String,
    pub(crate) file_size: i64,
    pub(crate) modified_at: String,
    pub(crate) lrc_path: Option<String>,
    pub(crate) change: TrackChange,
    pub(crate) metadata: Option<MetadataRead>,
}

#[derive(Debug)]
pub(crate) struct ScanProblem {
    pub(crate) path: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ScanRepository {
    pool: SqlitePool,
}

impl ScanRepository {
    pub(crate) const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub(crate) async fn create_run(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query("INSERT INTO scans DEFAULT VALUES")
            .execute(&self.pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    pub(crate) async fn load_tracks(&self) -> Result<Vec<StoredTrack>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT id, path, file_size, modified_at, missing_since FROM tracks")
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter()
            .map(|row| {
                Ok(StoredTrack {
                    id: row.try_get("id")?,
                    path: row.try_get("path")?,
                    file_size: row.try_get("file_size")?,
                    modified_at: row.try_get("modified_at")?,
                    missing_since: row.try_get("missing_since")?,
                })
            })
            .collect()
    }

    pub(crate) async fn complete(
        &self,
        tracks: &[ScannedTrack],
        missing_track_ids: &[i64],
        problems: &[ScanProblem],
        summary: ScanSummary,
    ) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin().await?;

        for track in tracks {
            let track_id = apply_track_change(&mut transaction, track).await?;
            sync_lrc_presence(&mut transaction, track_id, track.lrc_path.as_deref()).await?;
            if let Some(metadata) = &track.metadata {
                replace_metadata(&mut transaction, track_id, metadata).await?;
            }
        }

        for track_id in missing_track_ids {
            sqlx::query(
                "UPDATE tracks \
             SET missing_since = COALESCE(missing_since, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ?",
            )
            .bind(track_id)
            .execute(&mut *transaction)
            .await?;
        }

        for problem in problems {
            sqlx::query(
                "INSERT INTO scan_errors (scan_id, track_id, path, error_message) \
             VALUES (?, (SELECT id FROM tracks WHERE path = ?), ?, ?)",
            )
            .bind(summary.scan_id())
            .bind(&problem.path)
            .bind(&problem.path)
            .bind(&problem.message)
            .execute(&mut *transaction)
            .await?;
        }

        let status = if summary.errors() == 0 {
            "completed"
        } else {
            "completed_with_errors"
        };
        sqlx::query(
            "UPDATE scans SET \
             finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
             status = ?, discovered_tracks = ?, processed_tracks = ?, \
             new_tracks = ?, updated_tracks = ?, unchanged_tracks = ?, \
             removed_tracks = ?, error_count = ? \
         WHERE id = ?",
        )
        .bind(status)
        .bind(summary.discovered())
        .bind(summary.processed())
        .bind(summary.new_tracks())
        .bind(summary.updated_tracks())
        .bind(summary.unchanged_tracks())
        .bind(summary.removed_tracks())
        .bind(summary.errors())
        .bind(summary.scan_id())
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await
    }

    pub(crate) async fn fail(&self, scan_id: i64, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE scans SET \
             finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
             status = 'failed', error_message = ? \
         WHERE id = ?",
        )
        .bind(error)
        .bind(scan_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

async fn replace_metadata(
    transaction: &mut Transaction<'_, Sqlite>,
    track_id: i64,
    result: &MetadataRead,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM track_metadata_issues WHERE track_id = ?")
        .bind(track_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM track_metadata_values WHERE track_id = ?")
        .bind(track_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM track_metadata WHERE track_id = ?")
        .bind(track_id)
        .execute(&mut **transaction)
        .await?;

    match result {
        MetadataRead::Complete(metadata) => {
            insert_metadata(transaction, track_id, metadata).await?;
        }
        MetadataRead::Failed(issue) => {
            insert_metadata_issue(transaction, track_id, issue).await?;
        }
    }

    Ok(())
}

async fn insert_metadata(
    transaction: &mut Transaction<'_, Sqlite>,
    track_id: i64,
    metadata: &TrackMetadata,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO track_metadata (track_id, title, album, duration_ms, file_format) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(track_id)
    .bind(&metadata.title)
    .bind(&metadata.album)
    .bind(metadata.duration_ms)
    .bind(metadata.file_format)
    .execute(&mut **transaction)
    .await?;

    let mut positions = std::collections::HashMap::new();
    for value in &metadata.values {
        let position = positions.entry(value.field.as_str()).or_insert(0_i64);
        sqlx::query(
            "INSERT INTO track_metadata_values (track_id, field, position, value) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(track_id)
        .bind(value.field.as_str())
        .bind(*position)
        .bind(&value.value)
        .execute(&mut **transaction)
        .await?;
        *position += 1;
    }

    for issue in &metadata.issues {
        insert_metadata_issue(transaction, track_id, issue).await?;
    }

    Ok(())
}

async fn insert_metadata_issue(
    transaction: &mut Transaction<'_, Sqlite>,
    track_id: i64,
    issue: &crate::metadata::MetadataIssue,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO track_metadata_issues (track_id, field, kind, message) VALUES (?, ?, ?, ?)",
    )
    .bind(track_id)
    .bind(issue.field)
    .bind(issue.kind)
    .bind(&issue.message)
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

async fn apply_track_change(
    transaction: &mut Transaction<'_, Sqlite>,
    track: &ScannedTrack,
) -> Result<i64, sqlx::Error> {
    match track.change {
        TrackChange::New => {
            let result =
                sqlx::query("INSERT INTO tracks (path, file_size, modified_at) VALUES (?, ?, ?)")
                    .bind(&track.path)
                    .bind(track.file_size)
                    .bind(&track.modified_at)
                    .execute(&mut **transaction)
                    .await?;

            Ok(result.last_insert_rowid())
        }
        TrackChange::Changed => {
            let track_id = existing_track_id(track)?;
            sqlx::query(
                "UPDATE tracks SET file_size = ?, modified_at = ?, missing_since = NULL, \
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ?",
            )
            .bind(track.file_size)
            .bind(&track.modified_at)
            .bind(track_id)
            .execute(&mut **transaction)
            .await?;

            Ok(track_id)
        }
        TrackChange::Unchanged => existing_track_id(track),
    }
}

fn existing_track_id(track: &ScannedTrack) -> Result<i64, sqlx::Error> {
    track.id.ok_or_else(|| {
        sqlx::Error::Protocol(format!("{} track is missing its database id", track.path))
    })
}

async fn sync_lrc_presence(
    transaction: &mut Transaction<'_, Sqlite>,
    track_id: i64,
    lrc_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    if let Some(lrc_path) = lrc_path {
        sqlx::query(
            "INSERT INTO track_lyrics (track_id, status, lrc_path) \
             VALUES (?, 'existing', ?) \
             ON CONFLICT(track_id) DO UPDATE SET \
                 status = CASE \
                     WHEN track_lyrics.status = 'written' THEN 'written' \
                     ELSE 'existing' \
                 END, \
                 lrc_path = excluded.lrc_path, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(track_id)
        .bind(lrc_path)
        .execute(&mut **transaction)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO track_lyrics (track_id, status) VALUES (?, 'pending') \
             ON CONFLICT(track_id) DO UPDATE SET \
                 status = 'pending', lrc_path = NULL, lyrics_format = NULL, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE track_lyrics.status IN ('existing', 'written')",
        )
        .bind(track_id)
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}
