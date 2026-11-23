use std::path::Path;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::config::models::{
    AppConfigDto, PathValidationResponse, UpdateConfigRequest, ValidatePathRequest,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_config).post(update_config))
        .route("/validate-path", post(validate_path))
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<AppConfigDto>, AppError> {
    let cfg = AppConfigDto {
        downloads_dir: state.config.downloads_dir.to_string_lossy().to_string(),
        library_dir: state.config.library_dir.to_string_lossy().to_string(),
        covers_dir: state.config.covers_dir.to_string_lossy().to_string(),
        temp_encode_dir: state.config.temp_encode_dir.to_string_lossy().to_string(),
        nyaa_base_url: state.config.nyaa_base_url.clone(),
        nyaa_default_category: state.config.nyaa_default_category.clone(),
        anilist_graphql_url: state.config.anilist_graphql_url.clone(),
        torrent_listen_port: state.config.torrent_listen_port,
        torrent_enable_dht: state.config.torrent_enable_dht,
        torrent_global_download_limit_kbps: state.config.torrent_global_download_limit_kbps,
        torrent_global_upload_limit_kbps: state.config.torrent_global_upload_limit_kbps,
        ffmpeg_binary_path: state.config.ffmpeg_binary_path.clone(),
        ffprobe_binary_path: state.config.ffprobe_binary_path.clone(),
        encoder_default_preset: state.config.encoder_default_preset.clone(),
    };

    Ok(Json(cfg))
}

async fn update_config(
    State(state): State<AppState>,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Validate directory existence if provided
    if let Some(dl) = &payload.downloads_dir {
        let p = Path::new(dl);
        if !p.exists() {
            tokio::fs::create_dir_all(p).await.map_err(|e| {
                AppError::BadRequest(format!("Could not create or access downloads directory: {e}"))
            })?;
        }
    }

    if let Some(lib) = &payload.library_dir {
        let p = Path::new(lib);
        if !p.exists() {
            tokio::fs::create_dir_all(p).await.map_err(|e| {
                AppError::BadRequest(format!("Could not create or access library directory: {e}"))
            })?;
        }
    }

    // Persist into app_config table
    let config_val = json!({
        "downloads_dir": payload.downloads_dir,
        "library_dir": payload.library_dir,
        "nyaa_default_category": payload.nyaa_default_category,
        "torrent_global_download_limit_kbps": payload.torrent_global_download_limit_kbps,
        "torrent_global_upload_limit_kbps": payload.torrent_global_upload_limit_kbps,
        "encoder_default_preset": payload.encoder_default_preset,
    });

    sqlx::query(
        r#"
        INSERT INTO app_config (key, value, updated_at)
        VALUES ('settings', $1, NOW())
        ON CONFLICT (key) DO UPDATE SET
            value = $1,
            updated_at = NOW()
        "#
    )
    .bind(config_val)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Configuration updated successfully"
    })))
}

async fn validate_path(
    Json(payload): Json<ValidatePathRequest>,
) -> Result<Json<PathValidationResponse>, AppError> {
    let p = Path::new(&payload.path);
    let exists = p.exists();
    let is_dir = p.is_dir();

    let writable = if exists && is_dir {
        let test_file = p.join(format!(".nyaalist_write_test_{}", uuid::Uuid::new_v4()));
        match tokio::fs::write(&test_file, b"ok").await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(&test_file).await;
                true
            }
            Err(_) => false,
        }
    } else {
        false
    };

    Ok(Json(PathValidationResponse {
        path: payload.path,
        exists,
        is_dir,
        writable,
    }))
}
