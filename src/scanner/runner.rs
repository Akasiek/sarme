use std::path::{Path, PathBuf};

use super::error::ScanError;
use super::service;
use super::summary::ScanSummary;
use crate::database::scans::ScanRepository;

#[derive(Clone, Debug)]
pub(crate) struct Scanner {
    library_root: PathBuf,
    repository: ScanRepository,
}

impl Scanner {
    pub(crate) fn new(library_root: PathBuf, repository: ScanRepository) -> Self {
        Self {
            library_root,
            repository,
        }
    }

    pub(crate) async fn scan(&self) -> Result<ScanSummary, ScanError> {
        service::scan(self).await
    }

    pub(super) fn library_root(&self) -> &Path {
        &self.library_root
    }

    pub(super) const fn repository(&self) -> &ScanRepository {
        &self.repository
    }
}
