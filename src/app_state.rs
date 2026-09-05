use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    config: Arc<AppConfig>,
}

impl AppState {
    pub(crate) fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub(crate) fn config(&self) -> &AppConfig {
        &self.config
    }
}
