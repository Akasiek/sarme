use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use super::error::DatabaseError;
use crate::config::AppConfig;

pub(crate) async fn connect(config: &AppConfig) -> Result<SqlitePool, DatabaseError> {
    create_data_directory(config)?;
    connect_url(config.database_url()).await
}

fn create_data_directory(config: &AppConfig) -> Result<(), DatabaseError> {
    std::fs::create_dir_all(config.data_dir()).map_err(|source| {
        DatabaseError::CreateDataDirectory {
            path: config.data_dir().to_path_buf(),
            source,
        }
    })?;
    Ok(())
}

pub(super) async fn connect_url(database_url: &str) -> Result<SqlitePool, DatabaseError> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(DatabaseError::InvalidUrl)?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(DatabaseError::Connect)?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(DatabaseError::Migrate)?;

    Ok(pool)
}
