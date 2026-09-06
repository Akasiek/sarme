use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum MetadataReadError {
    #[error("could not read audio metadata from {path}: {source}")]
    File {
        path: PathBuf,
        #[source]
        source: lofty::error::FileParseError,
    },
    #[error("audio duration for {path} exceeds SQLite integer range")]
    DurationOutOfRange { path: PathBuf },
}
