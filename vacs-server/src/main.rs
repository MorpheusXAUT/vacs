use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::watch;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use vacs_server::auth::layer::setup_auth_layer;
use vacs_server::build::BuildInfo;
use vacs_server::config::AppConfig;
use vacs_server::release::catalog::file::FileCatalog;
use vacs_server::release::policy::Policy;
use vacs_server::release::UpdateChecker;
use vacs_server::routes::create_app;
use vacs_server::state::AppState;
use vacs_server::store::redis::RedisStore;
use vacs_server::store::Store;
use vacs_vatsim::slurper::SlurperClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=trace,vacs_=trace,tower_http=debug,tower_sessions=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let build_info = BuildInfo::gather();
    tracing::info!(?build_info);

    let config = AppConfig::parse()?;

    let catalog = FileCatalog::new(&config.updates.release_manifest_path)?;
    let policy = Policy::new(&config.updates.policy_path)?;
    let updates = UpdateChecker::new(Arc::new(catalog), policy);

    let redis_store = RedisStore::new(&config.redis).await?;
    let redis_pool = redis_store.get_pool().clone();

    let slurper = SlurperClient::new(config.vatsim.slurper_base_url.as_str())?;

    let (shutdown_tx, shutdown_rx) = watch::channel(());

    let app_state = Arc::new(AppState::new(
        config.clone(),
        updates,
        Store::Redis(redis_store),
        slurper,
        shutdown_rx.clone(),
    ));

    let auth_layer = setup_auth_layer(&config, redis_pool).await?;

    let app = create_app(auth_layer);

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
