use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::database::scans::ScanRepository;
use crate::lrclib::LrclibClient;
use crate::scanner::Scanner;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    config: Arc<AppConfig>,
    #[expect(dead_code, reason = "used by the upcoming persistence repositories")]
    database: SqlitePool,
    #[expect(dead_code, reason = "used by the upcoming lyrics processing queue")]
    lrclib: LrclibClient,
    scanner: Scanner,
}

impl AppState {
    pub(crate) fn new(
        config: AppConfig,
        database: SqlitePool,
    ) -> Result<Self, crate::lrclib::LrclibError> {
        let repository = ScanRepository::new(database.clone());
        let scanner = Scanner::new(config.library_dir().to_path_buf(), repository);

        Ok(Self {
            config: Arc::new(config),
            database,
            lrclib: LrclibClient::new()?,
            scanner,
        })
    }

    pub(crate) fn config(&self) -> &AppConfig {
        &self.config
    }

    #[expect(dead_code, reason = "used by the upcoming scan queue")]
    pub(crate) fn scanner(&self) -> &Scanner {
        &self.scanner
    }
}
