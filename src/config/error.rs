use std::net::AddrParseError;
use std::num::ParseIntError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("{name} is not set or is empty")]
    MissingVariable { name: &'static str },

    #[error("{name} contains non-UTF-8 data")]
    InvalidUnicode { name: &'static str },

    #[error("{name} must be an absolute path, got {}", path.display())]
    DirectoryNotAbsolute { name: &'static str, path: PathBuf },

    #[error("directory configured by {name} at {} is not accessible: {source}", path.display())]
    DirectoryUnavailable {
        name: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("path configured by {name} at {} is not a directory", path.display())]
    PathNotDirectory { name: &'static str, path: PathBuf },

    #[error("{name} must be a valid IP address, got {value}: {source}")]
    InvalidHost {
        name: &'static str,
        value: String,
        #[source]
        source: AddrParseError,
    },

    #[error("{name} must be a valid integer, got {value}: {source}")]
    InvalidInteger {
        name: &'static str,
        value: String,
        #[source]
        source: ParseIntError,
    },

    #[error("{name} must be greater than zero")]
    ZeroNotAllowed { name: &'static str },
}
