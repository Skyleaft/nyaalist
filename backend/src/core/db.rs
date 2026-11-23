use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use crate::core::config::AppConfig;
use crate::core::error::AppError;

pub async fn create_db_pool(config: &AppConfig) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(Duration::from_secs(config.database_timeout_seconds))
        .connect(&config.database_url)
        .await
        .map_err(AppError::Database)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database migration failed: {}", e)))
}
