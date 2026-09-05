use std::path::PathBuf;

use sqlx::migrate::MigrateError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DatabaseError {
    #[error("could not create application data directory at {}: {source}", path.display())]
    CreateDataDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid SQLite database URL: {0}")]
    InvalidUrl(#[source] sqlx::Error),

    #[error("could not connect to SQLite: {0}")]
    Connect(#[source] sqlx::Error),

    #[error("could not apply SQLite migrations: {0}")]
    Migrate(#[source] MigrateError),
}
