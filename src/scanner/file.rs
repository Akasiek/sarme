use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::error::ScanIssue;

#[derive(Debug)]
pub(crate) struct DiscoveredFile {
    absolute_path: PathBuf,
    relative_path: PathBuf,
    file_size: u64,
    modified_at: SystemTime,
    lrc_path: Option<PathBuf>,
}

impl DiscoveredFile {
    pub(super) fn new(
        absolute_path: PathBuf,
        relative_path: PathBuf,
        file_size: u64,
        modified_at: SystemTime,
        lrc_path: Option<PathBuf>,
    ) -> Self {
        Self {
            absolute_path,
            relative_path,
            file_size,
            modified_at,
            lrc_path,
        }
    }

    pub(crate) fn absolute_path(&self) -> &Path {
        &self.absolute_path
    }

    pub(crate) fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub(crate) const fn file_size(&self) -> u64 {
        self.file_size
    }

    pub(crate) const fn modified_at(&self) -> SystemTime {
        self.modified_at
    }

    pub(crate) fn lrc_path(&self) -> Option<&Path> {
        self.lrc_path.as_deref()
    }
}

#[derive(Debug)]
pub(crate) struct Discovery {
    root: PathBuf,
    files: Vec<DiscoveredFile>,
    issues: Vec<ScanIssue>,
}

impl Discovery {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Vec::new(),
            issues: Vec::new(),
        }
    }

    pub(super) fn add_file(&mut self, file: DiscoveredFile) {
        self.files.push(file);
    }

    pub(super) fn add_issue(&mut self, issue: ScanIssue) {
        self.issues.push(issue);
    }

    pub(super) fn sort_files(&mut self) {
        self.files
            .sort_unstable_by(|left, right| left.relative_path.cmp(&right.relative_path));
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn files(&self) -> &[DiscoveredFile] {
        &self.files
    }

    pub(crate) fn issues(&self) -> &[ScanIssue] {
        &self.issues
    }
}
