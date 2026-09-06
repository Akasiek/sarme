mod app_state;
mod config;
mod database;
mod lrclib;
mod metadata;
mod scanner;
mod web;

use app_state::AppState;

/// Loads configuration and runs the application.
///
/// # Errors
///
/// Returns an error when the application configuration is invalid or a service
/// required by the application cannot be started.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    web::tracer::init();

    let config = config::load()?;
    let database = database::connect(&config).await?;
    let state = AppState::new(config, database)?;

    web::run(state).await?;

    Ok(())
}
