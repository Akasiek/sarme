use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug)]
pub(crate) struct AppConfig {
    pub(super) library_dir: PathBuf,
    pub(super) data_dir: PathBuf,
    pub(super) database_url: String,
    pub(super) listen_address: SocketAddr,
    #[expect(dead_code, reason = "used by the upcoming scheduled scanning service")]
    pub(super) scan_interval: Duration,
}

impl AppConfig {
    pub(crate) fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    pub(crate) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub(crate) fn database_url(&self) -> &str {
        &self.database_url
    }

    pub(crate) const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }
}
