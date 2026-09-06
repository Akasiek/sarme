use std::fs;
use std::path::Path;

use super::discovery::discover;
use super::error::DiscoveryError;

#[test]
fn discovers_supported_audio_files_and_existing_lrc_files() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempfile::tempdir()?;
    let album = directory.path().join("Björk").join("Debut");
    fs::create_dir_all(&album)?;

    for filename in [
        "Human Behaviour.FLAC",
        "Crying.mp3",
        "Venus as a Boy.opus",
        "There's More to Life Than This.ogg",
        "Like Someone in Love.m4a",
    ] {
        fs::write(album.join(filename), b"audio")?;
    }
    fs::write(album.join("ignored.wav"), b"audio")?;
    fs::write(album.join("Human Behaviour.lrc"), b"[00:01.00]Test")?;

    let discovery = discover(directory.path())?;

    assert_eq!(discovery.files().len(), 5);
    assert!(discovery.issues().is_empty());
    assert_eq!(discovery.root(), directory.path().canonicalize()?);
    assert!(discovery.files().iter().all(|file| {
        file.absolute_path().starts_with(discovery.root())
            && file.file_size() == 5
            && file.modified_at() <= std::time::SystemTime::now()
    }));
    let track_with_lrc = discovery
        .files()
        .iter()
        .find(|file| file.relative_path() == Path::new("Björk/Debut/Human Behaviour.FLAC"));

    assert!(track_with_lrc.is_some_and(|file| {
        file.lrc_path() == Some(album.join("Human Behaviour.lrc").as_path())
    }));

    Ok(())
}

#[test]
fn rejects_an_unavailable_library() {
    let path = std::env::temp_dir().join("sarme-scanner-directory-that-does-not-exist");
    let result = discover(&path);

    assert!(matches!(
        result,
        Err(DiscoveryError::LibraryUnavailable {
            path: error_path,
            ..
        }) if error_path == path
    ));
}

#[test]
fn rejects_a_file_as_the_library_root() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("track.flac");
    fs::write(&path, b"audio")?;

    let result = discover(&path);

    assert!(matches!(
        result,
        Err(DiscoveryError::LibraryNotDirectory { path: error_path }) if error_path == path
    ));

    Ok(())
}

#[cfg(unix)]
#[test]
fn accepts_a_symlink_as_the_configured_root() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir()?;
    let library = directory.path().join("library");
    let configured_root = directory.path().join("music");
    fs::create_dir(&library)?;
    fs::write(library.join("track.flac"), b"audio")?;
    symlink(&library, &configured_root)?;

    let discovery = discover(&configured_root)?;

    assert_eq!(discovery.root(), library.canonicalize()?);
    assert_eq!(discovery.files().len(), 1);

    Ok(())
}

#[cfg(unix)]
#[test]
fn does_not_follow_symlinks_inside_the_library() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let library = tempfile::tempdir()?;
    let external = tempfile::tempdir()?;
    fs::write(library.path().join("local.flac"), b"audio")?;
    fs::write(external.path().join("external.flac"), b"audio")?;
    symlink(external.path(), library.path().join("external"))?;
    symlink(
        external.path().join("external.flac"),
        library.path().join("linked.flac"),
    )?;

    let discovery = discover(library.path())?;

    assert_eq!(discovery.files().len(), 1);
    assert_eq!(
        discovery.files().first().map(|file| file.relative_path()),
        Some(Path::new("local.flac"))
    );

    Ok(())
}

#[test]
fn scan_issue_exposes_its_path_and_message() {
    let path = std::path::PathBuf::from("track.flac");
    let error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let issue = super::error::ScanIssue::from_io(&path, &error);

    assert_eq!(issue.path(), Some(Path::new("track.flac")));
    assert!(issue.message().contains("denied"));
}

#[tokio::test]
async fn persists_incremental_scan_state() -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::Row;
    use sqlx::sqlite::SqlitePoolOptions;

    let library = tempfile::tempdir()?;
    let database = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!().run(&database).await?;

    let first_track = library.path().join("first.flac");
    let second_track = library.path().join("second.mp3");
    fs::write(&first_track, b"first")?;
    fs::write(&second_track, b"second")?;
    fs::write(library.path().join("first.lrc"), b"[00:01.00]Test")?;

    let repository = crate::database::scans::ScanRepository::new(database.clone());
    let scanner = super::Scanner::new(library.path().to_path_buf(), repository);
    let first_scan = scanner.scan().await?;
    assert_eq!(first_scan.scan_id(), 1);
    assert_eq!(first_scan.new_tracks(), 2);
    assert_eq!(first_scan.updated_tracks(), 0);
    assert_eq!(first_scan.unchanged_tracks(), 0);
    assert_eq!(first_scan.removed_tracks(), 0);

    let existing_lyrics = sqlx::query(
        "SELECT track_lyrics.status, track_lyrics.lrc_path \
         FROM track_lyrics JOIN tracks ON tracks.id = track_lyrics.track_id \
         WHERE tracks.path = 'first.flac'",
    )
    .fetch_one(&database)
    .await?;
    assert_eq!(existing_lyrics.try_get::<String, _>("status")?, "existing");
    assert_eq!(
        existing_lyrics.try_get::<String, _>("lrc_path")?,
        "first.lrc"
    );
    let pending_status: String = sqlx::query_scalar(
        "SELECT track_lyrics.status \
         FROM track_lyrics JOIN tracks ON tracks.id = track_lyrics.track_id \
         WHERE tracks.path = 'second.mp3'",
    )
    .fetch_one(&database)
    .await?;
    assert_eq!(pending_status, "pending");

    let unchanged_scan = scanner.scan().await?;
    assert_eq!(unchanged_scan.new_tracks(), 0);
    assert_eq!(unchanged_scan.updated_tracks(), 0);
    assert_eq!(unchanged_scan.unchanged_tracks(), 2);

    fs::remove_file(&second_track)?;
    let removal_scan = scanner.scan().await?;
    assert_eq!(removal_scan.unchanged_tracks(), 1);
    assert_eq!(removal_scan.removed_tracks(), 1);
    let missing_since: Option<String> =
        sqlx::query_scalar("SELECT missing_since FROM tracks WHERE path = 'second.mp3'")
            .fetch_one(&database)
            .await?;
    assert!(missing_since.is_some());

    fs::write(&second_track, b"second restored")?;
    let restored_scan = scanner.scan().await?;
    assert_eq!(restored_scan.updated_tracks(), 1);
    let restored_missing_since: Option<String> =
        sqlx::query_scalar("SELECT missing_since FROM tracks WHERE path = 'second.mp3'")
            .fetch_one(&database)
            .await?;
    assert!(restored_missing_since.is_none());

    Ok(())
}

#[tokio::test]
async fn records_a_failed_scan_when_the_library_is_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
    use sqlx::sqlite::SqlitePoolOptions;

    let database = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!().run(&database).await?;
    let missing_library =
        std::env::temp_dir().join("sarme-scanner-failed-scan-directory-that-does-not-exist");

    let repository = crate::database::scans::ScanRepository::new(database.clone());
    let scanner = super::Scanner::new(missing_library, repository);
    let result = scanner.scan().await;

    assert!(result.is_err());
    let status: String = sqlx::query_scalar("SELECT status FROM scans")
        .fetch_one(&database)
        .await?;
    assert_eq!(status, "failed");

    Ok(())
}

#[tokio::test]
async fn persists_metadata_without_refreshing_an_unchanged_track()
-> Result<(), Box<dyn std::error::Error>> {
    use sqlx::Row;
    use sqlx::sqlite::SqlitePoolOptions;

    let library = tempfile::tempdir()?;
    crate::metadata::fixtures::write(library.path(), "flac")?;
    let database = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!().run(&database).await?;
    let repository = crate::database::scans::ScanRepository::new(database.clone());
    let scanner = super::Scanner::new(library.path().to_path_buf(), repository);

    scanner.scan().await?;

    let metadata = sqlx::query("SELECT title, album, duration_ms, file_format FROM track_metadata")
        .fetch_one(&database)
        .await?;
    assert_eq!(metadata.try_get::<String, _>("title")?, "Fixture Song");
    assert_eq!(metadata.try_get::<String, _>("album")?, "Fixture Album");
    assert!(metadata.try_get::<i64, _>("duration_ms")? > 0);
    assert_eq!(metadata.try_get::<String, _>("file_format")?, "flac");
    let values: Vec<(String, String)> =
        sqlx::query_as("SELECT field, value FROM track_metadata_values ORDER BY field, position")
            .fetch_all(&database)
            .await?;
    assert!(values.contains(&("artist".to_owned(), "First Artist".to_owned())));
    assert!(values.contains(&("genre".to_owned(), "Rock".to_owned())));

    sqlx::query("UPDATE track_metadata SET title = 'Do not refresh'")
        .execute(&database)
        .await?;
    let second_scan = scanner.scan().await?;
    let title: String = sqlx::query_scalar("SELECT title FROM track_metadata")
        .fetch_one(&database)
        .await?;

    assert_eq!(second_scan.unchanged_tracks(), 1);
    assert_eq!(title, "Do not refresh");

    let replacement = tempfile::tempdir()?;
    let replacement_path = crate::metadata::fixtures::write(replacement.path(), "flac")?;
    fs::copy(replacement_path, library.path().join("fixture.flac"))?;
    let changed_scan = scanner.scan().await?;
    let refreshed = sqlx::query("SELECT title, file_format FROM track_metadata")
        .fetch_one(&database)
        .await?;

    assert_eq!(changed_scan.updated_tracks(), 1);
    assert_eq!(refreshed.try_get::<String, _>("title")?, "Fixture Song");
    assert_eq!(refreshed.try_get::<String, _>("file_format")?, "flac");
    Ok(())
}

#[tokio::test]
async fn persists_missing_metadata_as_track_issues() -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::sqlite::SqlitePoolOptions;

    let library = tempfile::tempdir()?;
    crate::metadata::fixtures::write(library.path(), "untagged.flac")?;
    let database = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!().run(&database).await?;
    let repository = crate::database::scans::ScanRepository::new(database.clone());
    let scanner = super::Scanner::new(library.path().to_path_buf(), repository);

    let summary = scanner.scan().await?;
    let title: Option<String> = sqlx::query_scalar("SELECT title FROM track_metadata")
        .fetch_one(&database)
        .await?;
    let issue_fields: Vec<String> = sqlx::query_scalar(
        "SELECT field FROM track_metadata_issues WHERE kind = 'missing' ORDER BY field",
    )
    .fetch_all(&database)
    .await?;

    assert_eq!(summary.errors(), 0);
    assert!(title.is_none());
    assert_eq!(issue_fields, vec!["album", "artist", "title"]);

    let replacement = tempfile::tempdir()?;
    let replacement_path = crate::metadata::fixtures::write(replacement.path(), "flac")?;
    fs::copy(
        replacement_path,
        library.path().join("fixture.untagged.flac"),
    )?;
    let changed_scan = scanner.scan().await?;
    let refreshed_title: String = sqlx::query_scalar("SELECT title FROM track_metadata")
        .fetch_one(&database)
        .await?;
    let remaining_issues: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_metadata_issues")
        .fetch_one(&database)
        .await?;

    assert_eq!(changed_scan.updated_tracks(), 1);
    assert_eq!(refreshed_title, "Fixture Song");
    assert_eq!(remaining_issues, 0);
    Ok(())
}
