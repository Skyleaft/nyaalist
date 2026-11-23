use std::sync::Arc;
use sqlx::PgPool;
use crate::core::config::AppConfig;
use crate::core::ws::WsHub;
use crate::features::downloader::TorrentEngine;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub ws: WsHub,
    pub torrent_engine: Arc<TorrentEngine>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        db: PgPool,
        ws: WsHub,
        torrent_engine: Arc<TorrentEngine>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db,
            ws,
            torrent_engine,
        }
    }
}
