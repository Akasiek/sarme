#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrackMetadata {
    pub(crate) title: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) duration_ms: i64,
    pub(crate) file_format: &'static str,
    pub(crate) values: Vec<MetadataValue>,
    pub(crate) issues: Vec<MetadataIssue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MetadataField {
    Artist,
    AlbumArtist,
    Genre,
}

impl MetadataField {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Artist => "artist",
            Self::AlbumArtist => "album_artist",
            Self::Genre => "genre",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataValue {
    pub(crate) field: MetadataField,
    pub(crate) value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataIssue {
    pub(crate) field: Option<&'static str>,
    pub(crate) kind: &'static str,
    pub(crate) message: String,
}

impl MetadataIssue {
    pub(super) fn missing(field: &'static str) -> Self {
        Self {
            field: Some(field),
            kind: "missing",
            message: format!("audio file is missing the {field} tag"),
        }
    }

    pub(crate) fn read_error(message: String) -> Self {
        Self {
            field: None,
            kind: "read_error",
            message,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MetadataRead {
    Complete(TrackMetadata),
    Failed(MetadataIssue),
}
