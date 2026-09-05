mod macros;
mod router;
mod routes;
mod server;
mod tracer;

pub(crate) async fn run() {
    tracer::init();
    server::run().await;
}
