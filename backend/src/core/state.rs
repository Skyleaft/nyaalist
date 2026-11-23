use std::sync::Arc;
use sea_orm::DatabaseConnection;
use sqlx::PgPool;
use crate::core::config::AppConfig;
use crate::core::ws::WsHub;
use crate::features::downloader::TorrentEngine;
use crate::features::explore::anilist_client::AniListClient;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub sea_db: DatabaseConnection,
    pub ws: WsHub,
    pub torrent_engine: Arc<TorrentEngine>,
    pub anilist_client: Arc<AniListClient>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        db: PgPool,
        sea_db: DatabaseConnection,
        ws: WsHub,
        torrent_engine: Arc<TorrentEngine>,
        anilist_client: Arc<AniListClient>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db,
            sea_db,
            ws,
            torrent_engine,
            anilist_client,
        }
    }
}
