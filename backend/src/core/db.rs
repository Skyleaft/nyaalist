use std::time::Duration;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::migration::Migrator;

/// Create raw SQLx connection pool for background drivers that require raw PgPool
pub async fn create_db_pool(config: &AppConfig) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .acquire_timeout(Duration::from_secs(config.database_timeout_seconds))
        .connect(&config.database_url)
        .await
        .map_err(AppError::Database)
}

/// Create SeaORM database connection pool
pub async fn create_sea_db_connection(config: &AppConfig) -> Result<DatabaseConnection, AppError> {
    let mut opt = ConnectOptions::new(&config.database_url);
    opt.max_connections(config.database_max_connections)
        .min_connections(config.database_min_connections)
        .connect_timeout(Duration::from_secs(config.database_timeout_seconds))
        .acquire_timeout(Duration::from_secs(config.database_timeout_seconds))
        .sqlx_logging(false);

    Database::connect(opt)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to connect to SeaORM database: {e}")))
}

/// Run automated SeaORM migrations on application first run
pub async fn run_sea_orm_migrations(db: &DatabaseConnection) -> Result<(), AppError> {
    Migrator::up(db, None)
        .await
        .map_err(|e| AppError::Internal(format!("SeaORM migration failed: {e}")))
}

/// Legacy SQLx migrations runner (backward-compatible fallback)
#[allow(dead_code)]
pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database migration failed: {}", e)))
}
