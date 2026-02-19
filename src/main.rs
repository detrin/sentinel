mod api;
mod auth;
mod config;
mod db;
mod models;
mod watchdog;
mod web;

use anyhow::Result;
use axum::{
    routing::{delete, get, post},
    Router,
};
use config::Config;
use std::sync::Arc;
use tokio::signal;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Sentinel...");

    let config = Config::from_env()?;
    info!("Configuration loaded");

    let pool = db::init_pool(&config.database_url).await?;
    info!("Database connected and migrations applied");

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
    };

    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/dashboard", get(web::dashboard::dashboard))
        .route("/switches/:id", get(web::dashboard::switch_detail))
        .route("/health", get(|| async { "OK" }))
        .route("/api/checkin/:id", post(api::checkin::checkin))
        .route("/api/switches", get(api::switches::list_switches))
        .route("/api/switches", post(api::switches::create_switch))
        .route("/api/switches/:id", get(api::switches::get_switch))
        .route("/api/switches/:id", delete(api::switches::delete_switch))
        .route("/api/scripts", get(api::switches::list_scripts))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
        )
        .with_state(state.clone());

    let watchdog_pool = pool.clone();
    let watchdog_config = state.config.clone();
    tokio::spawn(async move {
        watchdog::run_watchdog(watchdog_pool, watchdog_config).await;
    });

    let addr = config.server.bind_address.clone();
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn root_redirect() -> axum::response::Redirect {
    axum::response::Redirect::to("/dashboard")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}
