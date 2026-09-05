use crate::app_state::AppState;
use crate::web::router::get_app_router;
use listenfd::ListenFd;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

pub(super) async fn run(state: AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listen_address = state.config().listen_address();
    let router = get_app_router(state);
    let listener = get_app_listener(listen_address).await?;

    info!("Server listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router).await?;

    Ok(())
}

pub(super) async fn get_app_listener(listen_address: SocketAddr) -> std::io::Result<TcpListener> {
    let mut listener = ListenFd::from_env();

    if let Some(listener) = listener.take_tcp_listener(0)? {
        info!("Using listener from environment (e.g., systemd socket activation)");
        listener.set_nonblocking(true)?;
        TcpListener::from_std(listener)
    } else {
        TcpListener::bind(listen_address).await
    }
}
