use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;

use lofty::file::{AudioFile, FileType, TaggedFile, TaggedFileExt};
use lofty::tag::{Accessor, ItemKey, Tag};

use super::error::MetadataReadError;
use super::model::{MetadataField, MetadataIssue, MetadataValue, TrackMetadata};

pub(crate) fn read(path: &Path) -> Result<TrackMetadata, MetadataReadError> {
    let tagged = lofty::read_from_path(path).map_err(|source| MetadataReadError::File {
        path: path.to_path_buf(),
        source,
    })?;

    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let mut values = Vec::new();
    let mut issues = Vec::new();

    if let Some(tag) = tag {
        append_values(
            &mut values,
            MetadataField::Artist,
            tag,
            &[ItemKey::TrackArtist, ItemKey::TrackArtists],
        );
        append_values(
            &mut values,
            MetadataField::AlbumArtist,
            tag,
            &[ItemKey::AlbumArtist, ItemKey::AlbumArtists],
        );
        append_values(&mut values, MetadataField::Genre, tag, &[ItemKey::Genre]);
    }

    let title = get_value_from_tag(tag, Tag::title);
    let album = get_value_from_tag(tag, Tag::album);
    if title.is_none() {
        issues.push(MetadataIssue::missing("title"));
    }
    if album.is_none() {
        issues.push(MetadataIssue::missing("album"));
    }

    if !values
        .iter()
        .any(|value| value.field == MetadataField::Artist)
    {
        issues.push(MetadataIssue::missing("artist"));
    }

    Ok(TrackMetadata {
        title,
        album,
        duration_ms: get_duration_ms(path, &tagged)?,
        file_format: format_name(tagged.file_type()),
        values,
        issues,
    })
}

fn get_value_from_tag<'a>(
    tag: Option<&'a Tag>,
    accessor: impl FnOnce(&'a Tag) -> Option<Cow<'a, str>>,
) -> Option<String> {
    tag.and_then(accessor)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn append_values(
    output: &mut Vec<MetadataValue>,
    field: MetadataField,
    tag: &Tag,
    keys: &[ItemKey],
) {
    let mut seen = HashSet::new();
    for value in keys
        .iter()
        .flat_map(|key| tag.get_strings(*key))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if seen.insert(value.to_owned()) {
            output.push(MetadataValue {
                field,
                value: value.to_owned(),
            });
        }
    }
}

fn get_duration_ms(path: &Path, tagged: &TaggedFile) -> Result<i64, MetadataReadError> {
    let duration = tagged.properties().duration();
    let duration_ms =
        i64::try_from(duration.as_millis()).map_err(|_| MetadataReadError::DurationOutOfRange {
            path: path.to_path_buf(),
        })?;
    Ok(duration_ms)
}

fn format_name(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Flac => "flac",
        FileType::Mpeg => "mp3",
        FileType::Opus => "opus",
        FileType::Vorbis => "ogg",
        FileType::Mp4 => "m4a",
        _ => "unsupported",
    }
}
