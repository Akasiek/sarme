mod macros;
mod router;
mod routes;
mod server;
pub mod tracer;

pub(crate) async fn run() {
    server::run().await;
}
