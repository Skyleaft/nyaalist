mod core;
mod features;

use std::net::SocketAddr;
use axum::{
    routing::get,
    Json, Router,
};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::core::config::AppConfig;
use crate::core::db::create_db_pool;
use crate::core::state::AppState;
use crate::core::ws::WsHub;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,nyaalist=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load Configuration
    let config = AppConfig::from_env()?;
    tracing::info!("Initializing Nyaalist Backend on {}:{}", config.host, config.port);

    // 3. Ensure Storage Directories Exist
    tokio::fs::create_dir_all(&config.downloads_dir).await?;
    tokio::fs::create_dir_all(&config.library_dir).await?;
    tokio::fs::create_dir_all(&config.covers_dir).await?;
    tokio::fs::create_dir_all(&config.temp_encode_dir).await?;

    // 4. Connect to Database (gracefully handle initial offline state)
    let db_pool = match create_db_pool(&config).await {
        Ok(pool) => {
            tracing::info!("Connected to PostgreSQL database");
            pool
        }
        Err(err) => {
            tracing::warn!("Failed initial PostgreSQL connection ({}). Re-trying...", err);
            create_db_pool(&config).await?
        }
    };

    // 5. Initialize WebSocket Hub
    let ws_hub = WsHub::new(1024);

    // 6. Initialize BitTorrent Engine & Telemetry Monitor
    let torrent_engine = features::downloader::TorrentEngine::new(
        std::path::PathBuf::from(&config.downloads_dir),
        db_pool.clone(),
        ws_hub.clone(),
    )
    .await?;
    torrent_engine.clone().start_monitor();

    // 7. Build AppState
    let state = AppState::new(config.clone(), db_pool, ws_hub, torrent_engine);

    // 8. Configure Router & Middleware
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/ws", get(features::downloader::routes::ws_handler))
        .nest("/api/auth", features::auth::router())
        .nest("/api/explore", features::explore::router())
        .nest("/api/covers", features::explore::covers_router())
        .nest("/api/downloader", features::downloader::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 9. Bind & Serve
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Nyaalist API server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "nyaalist-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
