use anyhow::Context;
use config::{Config, Environment, File};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use vacs_server::app::create_app;
use vacs_server::config::AppConfig;
use vacs_server::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let config = load_config()?;

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let app_state = Arc::new(AppState::new(config.clone(), shutdown_rx.clone()));
    let app = create_app();

    let listener = tokio::net::TcpListener::bind(config.server.bind_addr).await?;

    tracing::info!(bind_addr = ?listener.local_addr()?, "Started listening");
    axum::serve(
        listener,
        app.with_state(app_state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_tx))
    .await?;

    Ok(())
}

fn load_config() -> anyhow::Result<AppConfig> {
    Config::builder()
        .set_default("server.bind_addr", "127.0.0.1:3000")?
        .set_default("auth.login_flow_timeout_millis", 10000)?
        .add_source(
            File::with_name(
                directories::ProjectDirs::from("app", "vacs", "vacs-server")
                    .expect("Failed to get project dirs")
                    .config_local_dir()
                    .join("config.toml")
                    .to_str()
                    .expect("Failed to get local config path"),
            )
            .required(false),
        )
        .add_source(File::with_name("config.toml").required(false))
        .add_source(Environment::with_prefix("vacs_server"))
        .build()
        .context("Failed to build config")?
        .try_deserialize()
        .context("Failed to deserialize config")
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
