use super::fixtures;
use super::model::MetadataField;
use super::read;

#[test]
fn reads_core_metadata_from_all_mvp_formats() -> Result<(), Box<dyn std::error::Error>> {
    for extension in ["flac", "mp3", "opus", "ogg", "m4a"] {
        let directory = tempfile::tempdir()?;
        let path = fixtures::write(directory.path(), extension)?;

        let metadata = read(&path)?;

        assert_eq!(metadata.title.as_deref(), Some("Fixture Song"));
        assert_eq!(metadata.album.as_deref(), Some("Fixture Album"));
        assert!(metadata.duration_ms > 0);
        assert!(metadata.issues.is_empty());
        assert!(metadata.values.iter().any(|value| {
            value.field == MetadataField::Artist && value.value == "First Artist"
        }));
        assert!(
            metadata
                .values
                .iter()
                .any(|value| value.field == MetadataField::Genre && value.value == "Rock")
        );
    }

    Ok(())
}

#[test]
fn records_missing_tags_without_using_the_filename() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = fixtures::write(directory.path(), "untagged.flac")?;

    let metadata = read(&path)?;

    assert!(metadata.title.is_none());
    assert!(metadata.album.is_none());
    assert!(metadata.values.is_empty());
    assert_eq!(metadata.issues.len(), 3);
    assert!(
        !metadata
            .issues
            .iter()
            .any(|issue| issue.message.contains("fixture"))
    );

    Ok(())
}
