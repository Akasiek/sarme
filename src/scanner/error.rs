use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub(crate) enum DiscoveryError {
    #[error("could not access music library at {}: {source}", path.display())]
    LibraryUnavailable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("music library path is not a directory: {}", path.display())]
    LibraryNotDirectory { path: PathBuf },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ScanError {
    #[error("could not persist library scan: {0}")]
    Database(#[from] sqlx::Error),

    #[error("could not run filesystem discovery task: {0}")]
    DiscoveryTask(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

#[derive(Debug)]
pub(crate) struct ScanIssue {
    path: Option<PathBuf>,
    message: String,
}

impl ScanIssue {
    pub(super) fn from_walkdir(error: &walkdir::Error) -> Self {
        let path = error.path().map(PathBuf::from);

        Self {
            path,
            message: error.to_string(),
        }
    }

    pub(super) fn from_io(path: &Path, error: &std::io::Error) -> Self {
        Self {
            path: Some(path.to_path_buf()),
            message: format!("could not inspect {}: {error}", path.display()),
        }
    }

    pub(super) fn invalid_path(path: &Path) -> Self {
        Self {
            path: Some(path.to_path_buf()),
            message: format!(
                "discovered path is outside the canonical music library root: {}",
                path.display()
            ),
        }
    }

    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}
