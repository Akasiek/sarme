use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::database::scans::ScanRepository;
use crate::scanner::Scanner;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    config: Arc<AppConfig>,
    #[expect(dead_code, reason = "used by the upcoming persistence repositories")]
    database: SqlitePool,
    scanner: Scanner,
}

impl AppState {
    pub(crate) fn new(config: AppConfig, database: SqlitePool) -> Self {
        let repository = ScanRepository::new(database.clone());
        let scanner = Scanner::new(config.library_dir().to_path_buf(), repository);

        Self {
            config: Arc::new(config),
            database,
            scanner,
        }
    }

    pub(crate) fn config(&self) -> &AppConfig {
        &self.config
    }

    #[expect(dead_code, reason = "used by the upcoming scan queue")]
    pub(crate) fn scanner(&self) -> &Scanner {
        &self.scanner
    }
}
