mod macros;
mod router;
mod routes;
mod server;
pub mod tracer;

use crate::app_state::AppState;

pub(crate) async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run(state).await
}
