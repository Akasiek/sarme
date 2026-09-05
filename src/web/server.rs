use crate::web::router::get_app_router;
use listenfd::ListenFd;
use tokio::net::TcpListener;
use tracing::info;

pub(super) async fn run() {
    let router = get_app_router();
    let listener = get_app_listener().await;

    info!(
        "Server listening on http://{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, router).await.unwrap();
}

pub async fn get_app_listener() -> TcpListener {
    let mut listener = ListenFd::from_env();
    match listener.take_tcp_listener(0).unwrap() {
        Some(listener) => {
            info!("Using listener from environment (e.g., systemd socket activation)");
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        // otherwise fall back to local listening
        None => {
            let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
            TcpListener::bind(host).await.unwrap()
        }
    }
}
