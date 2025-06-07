use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use vacs_server::app::create_app;
use vacs_server::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=trace,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let app_state = Arc::new(AppState::new(shutdown_rx.clone()));
    let app = create_app();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.with_state(app_state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_tx))
    .await
    .unwrap();
}

async fn shutdown_signal(shutdown_tx: watch::Sender<()>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("Shutdown signal received, terminating gracefully...");
    
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");
}
