mod core;
pub mod entities;
mod features;
pub mod migration;

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
use crate::core::db::{create_db_pool, create_sea_db_connection, run_sea_orm_migrations};
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

    // 4. Connect to Database (both raw PgPool and SeaORM connection)
    let db_pool = match create_db_pool(&config).await {
        Ok(pool) => {
            tracing::info!("Connected to PostgreSQL database (SQLx)");
            pool
        }
        Err(err) => {
            tracing::warn!("Failed initial PostgreSQL connection ({}). Re-trying...", err);
            create_db_pool(&config).await?
        }
    };

    let sea_db = match create_sea_db_connection(&config).await {
        Ok(conn) => {
            tracing::info!("Connected to PostgreSQL database via SeaORM");
            conn
        }
        Err(err) => {
            tracing::warn!("Failed initial SeaORM connection ({}). Re-trying...", err);
            create_sea_db_connection(&config).await?
        }
    };

    // Run SeaORM database migrations on first run
    tracing::info!("Verifying and executing SeaORM database migrations on startup...");
    if let Err(err) = run_sea_orm_migrations(&sea_db).await {
        tracing::warn!("SeaORM migration warning: {}", err);
    } else {
        tracing::info!("SeaORM database schema verified and up-to-date");
    }

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

    // 7. Initialize FFmpeg Video Transcoder Queue Worker
    let encoder_worker = features::encoder::EncodingQueueWorker::new(
        config.ffmpeg_binary_path.clone(),
        config.ffprobe_binary_path.clone(),
        db_pool.clone(),
        ws_hub.clone(),
    );
    encoder_worker.start();

    // 8. Build AppState with SeaORM connection
    let state = AppState::new(config.clone(), db_pool, sea_db, ws_hub, torrent_engine);

    // 9. Configure Router & Middleware
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
        .nest("/api/library", features::library::router())
        .nest("/api/encoder", features::encoder::router())
        .nest("/api/config", features::config::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 10. Bind & Serve
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
