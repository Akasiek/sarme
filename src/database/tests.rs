use sqlx::Row;

use super::connection::connect_url;

#[tokio::test]
async fn migrations_create_the_persistent_schema() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let database_path = directory.path().join("sarme.db");
    let database_url = format!("sqlite://{}", database_path.display());

    let pool = connect_url(&database_url).await?;
    let track_id = sqlx::query(
        "INSERT INTO tracks (path, file_size, modified_at, content_hash) \
         VALUES (?, ?, ?, ?)",
    )
    .bind("Artist/Album/Track.flac")
    .bind(1024_i64)
    .bind("2024-08-29T14:40:00.000Z")
    .bind("sha256:test")
    .execute(&pool)
    .await?
    .last_insert_rowid();
    let scan_id = sqlx::query(
        "INSERT INTO scans (status, discovered_tracks, processed_tracks, error_count) \
         VALUES ('completed_with_errors', 1, 1, 1)",
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO scan_errors (scan_id, track_id, path, error_message) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(scan_id)
    .bind(track_id)
    .bind("Artist/Album/Track.flac")
    .bind("metadata read failed")
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO track_metadata (track_id, title, album, duration_ms, file_format) \
         VALUES (?, 'Track', 'Album', 180000, 'flac')",
    )
    .bind(track_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO track_metadata_values (track_id, field, position, value) \
         VALUES (?, 'artist', 0, 'Artist'), (?, 'genre', 0, 'Rock')",
    )
    .bind(track_id)
    .bind(track_id)
    .execute(&pool)
    .await?;
    let lookup_attempt_id = sqlx::query(
        "INSERT INTO lyric_lookup_attempts \
         (track_id, status, query, candidate_count, next_retry_at) \
         VALUES (?, 'review', ?, 1, ?)",
    )
    .bind(track_id)
    .bind("Artist Track")
    .bind("2024-08-30T14:40:00.000Z")
    .execute(&pool)
    .await?
    .last_insert_rowid();
    let candidate_id = sqlx::query(
        "INSERT INTO lyric_review_candidates \
             (track_id, lookup_attempt_id, provider, provider_id, score, synced_lyrics) \
         VALUES (?, ?, 'lrclib', 'track-1', 82, ?)",
    )
    .bind(track_id)
    .bind(lookup_attempt_id)
    .bind("[00:01.00]Test")
    .execute(&pool)
    .await?
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO lyric_review_candidates \
             (track_id, lookup_attempt_id, provider, provider_id, score, plain_lyrics) \
         VALUES (?, ?, 'lrclib', 'track-plain', 75, ?)",
    )
    .bind(track_id)
    .bind(lookup_attempt_id)
    .bind("Ordinary lyrics")
    .execute(&pool)
    .await?;
    let empty_candidate = sqlx::query(
        "INSERT INTO lyric_review_candidates \
             (track_id, lookup_attempt_id, provider, provider_id, score) \
         VALUES (?, ?, 'lrclib', 'track-empty', 50)",
    )
    .bind(track_id)
    .bind(lookup_attempt_id)
    .execute(&pool)
    .await;
    assert!(empty_candidate.is_err());
    sqlx::query("INSERT INTO track_lyrics (track_id, status) VALUES (?, 'review')")
        .bind(track_id)
        .execute(&pool)
        .await?;

    let other_track_id =
        sqlx::query("INSERT INTO tracks (path, file_size, modified_at) VALUES (?, ?, ?)")
            .bind("Artist/Album/Other.flac")
            .bind(2048_i64)
            .bind("2024-08-29T14:41:00.000Z")
            .execute(&pool)
            .await?
            .last_insert_rowid();
    let mismatched_candidate = sqlx::query(
        "INSERT INTO lyric_review_candidates \
             (track_id, lookup_attempt_id, provider, provider_id, score, synced_lyrics) \
         VALUES (?, ?, 'lrclib', 'track-2', 90, ?)",
    )
    .bind(other_track_id)
    .bind(lookup_attempt_id)
    .bind("[00:01.00]Wrong track")
    .execute(&pool)
    .await;

    assert!(mismatched_candidate.is_err());
    drop(pool);

    let reopened_pool = connect_url(&database_url).await?;
    let track = sqlx::query(
        "SELECT path, modified_at, content_hash, created_at \
         FROM tracks WHERE id = ?",
    )
    .bind(track_id)
    .fetch_one(&reopened_pool)
    .await?;
    let scan_error = sqlx::query("SELECT error_message FROM scan_errors")
        .fetch_one(&reopened_pool)
        .await?;
    let lookup_attempt =
        sqlx::query("SELECT attempted_at, status, next_retry_at FROM lyric_lookup_attempts")
            .fetch_one(&reopened_pool)
            .await?;
    let track_lyrics = sqlx::query(
        "SELECT status, lrc_path FROM track_lyrics \
             WHERE track_id = ?",
    )
    .bind(track_id)
    .fetch_one(&reopened_pool)
    .await?;
    let metadata = sqlx::query(
        "SELECT title, album, duration_ms, file_format FROM track_metadata WHERE track_id = ?",
    )
    .bind(track_id)
    .fetch_one(&reopened_pool)
    .await?;
    let metadata_values: Vec<(String, String)> = sqlx::query_as(
        "SELECT field, value FROM track_metadata_values WHERE track_id = ? ORDER BY field",
    )
    .bind(track_id)
    .fetch_all(&reopened_pool)
    .await?;
    let review_candidate = sqlx::query(
        "SELECT id, synced_lyrics, plain_lyrics FROM lyric_review_candidates \
         WHERE track_id = ? AND provider_id = 'track-1'",
    )
    .bind(track_id)
    .fetch_one(&reopened_pool)
    .await?;
    let plain_candidate = sqlx::query(
        "SELECT synced_lyrics, plain_lyrics FROM lyric_review_candidates \
         WHERE track_id = ? AND provider_id = 'track-plain'",
    )
    .bind(track_id)
    .fetch_one(&reopened_pool)
    .await?;

    assert_eq!(
        track.try_get::<String, _>("path")?,
        "Artist/Album/Track.flac"
    );
    assert_eq!(track.try_get::<String, _>("content_hash")?, "sha256:test");
    assert_eq!(
        track.try_get::<String, _>("modified_at")?,
        "2024-08-29T14:40:00.000Z"
    );
    assert!(track.try_get::<String, _>("created_at")?.ends_with('Z'));
    assert_eq!(
        scan_error.try_get::<String, _>("error_message")?,
        "metadata read failed"
    );
    assert_eq!(lookup_attempt.try_get::<String, _>("status")?, "review");
    assert!(
        lookup_attempt
            .try_get::<String, _>("attempted_at")?
            .ends_with('Z')
    );
    assert_eq!(
        lookup_attempt.try_get::<String, _>("next_retry_at")?,
        "2024-08-30T14:40:00.000Z"
    );
    assert_eq!(track_lyrics.try_get::<String, _>("status")?, "review");
    assert_eq!(metadata.try_get::<String, _>("title")?, "Track");
    assert_eq!(metadata.try_get::<String, _>("album")?, "Album");
    assert_eq!(metadata.try_get::<i64, _>("duration_ms")?, 180_000);
    assert_eq!(metadata.try_get::<String, _>("file_format")?, "flac");
    assert_eq!(
        metadata_values,
        vec![
            ("artist".to_owned(), "Artist".to_owned()),
            ("genre".to_owned(), "Rock".to_owned()),
        ]
    );
    assert_eq!(review_candidate.try_get::<i64, _>("id")?, candidate_id);
    assert_eq!(
        review_candidate.try_get::<String, _>("synced_lyrics")?,
        "[00:01.00]Test"
    );
    assert!(
        review_candidate
            .try_get::<Option<String>, _>("plain_lyrics")?
            .is_none()
    );
    assert!(
        plain_candidate
            .try_get::<Option<String>, _>("synced_lyrics")?
            .is_none()
    );
    assert_eq!(
        plain_candidate.try_get::<String, _>("plain_lyrics")?,
        "Ordinary lyrics"
    );
    assert!(
        track_lyrics
            .try_get::<Option<String>, _>("lrc_path")?
            .is_none()
    );

    sqlx::query("DELETE FROM lyric_review_candidates WHERE track_id = ?")
        .bind(track_id)
        .execute(&reopened_pool)
        .await?;
    sqlx::query(
        "UPDATE track_lyrics \
         SET status = 'written', lrc_path = ?, lyrics_format = 'synced' \
         WHERE track_id = ?",
    )
    .bind("Artist/Album/Track.lrc")
    .bind(track_id)
    .execute(&reopened_pool)
    .await?;

    let retained_candidates: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lyric_review_candidates WHERE track_id = ?")
            .bind(track_id)
            .fetch_one(&reopened_pool)
            .await?;
    assert_eq!(retained_candidates, 0);

    Ok(())
}
