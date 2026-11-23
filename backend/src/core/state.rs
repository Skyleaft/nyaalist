use std::sync::Arc;
use sqlx::PgPool;
use crate::core::config::AppConfig;
use crate::core::ws::WsHub;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub ws: WsHub,
}

impl AppState {
    pub fn new(config: AppConfig, db: PgPool, ws: WsHub) -> Self {
        Self {
            config: Arc::new(config),
            db,
            ws,
        }
    }
}
