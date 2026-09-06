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
