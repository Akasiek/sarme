mod web;

/// Loads configuration and runs the application.
///
/// # Errors
///
/// Returns an error when the application configuration is invalid or a service
/// required by the application cannot be started.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    web::tracer::init();

    web::run().await;

    Ok(())
}
