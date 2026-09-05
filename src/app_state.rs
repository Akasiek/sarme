use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::AppConfig;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    config: Arc<AppConfig>,
    #[expect(dead_code, reason = "used by the upcoming persistence repositories")]
    database: SqlitePool,
}

impl AppState {
    pub(crate) fn new(config: AppConfig, database: SqlitePool) -> Self {
        Self {
            config: Arc::new(config),
            database,
        }
    }

    pub(crate) fn config(&self) -> &AppConfig {
        &self.config
    }
}
