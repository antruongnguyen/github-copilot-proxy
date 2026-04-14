use std::sync::Arc;

use clap::Parser;
use copilot_proxy::auth::AuthManager;
use copilot_proxy::config::Config;
use copilot_proxy::server::router;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let config = Config::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let auth = Arc::new(AuthManager::new(config.token_dir.clone()));
    let app = router::create_router(auth.clone(), config.api_key.clone());
    let addr = config.bind_addr();

    tracing::info!("Starting copilot-proxy on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Failed to bind to {addr}: {e}");
            std::process::exit(1);
        });

    // Spawn startup auth check — only open browser if device flow needed
    let auth_handle = auth.clone();
    let open_url = format!("http://{addr}");
    tokio::spawn(async move {
        let needs_auth = auth_handle.startup_auth().await;
        if needs_auth {
            tracing::info!("Opening browser for authentication: {open_url}");
            let _ = open::that(&open_url);
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Server error: {e}");
            std::process::exit(1);
        });

    tracing::info!("Server shut down");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");
    tracing::info!("Shutdown signal received");
}
