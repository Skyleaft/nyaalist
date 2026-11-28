use std::path::Path;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::entities::app_config;
use crate::features::config::models::{
    AppConfigDto, PathValidationResponse, UpdateConfigRequest, ValidatePathRequest,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_config).post(update_config))
        .route("/validate-path", post(validate_path))
}

pub fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}••••{suffix}")
}

/// Resolves the active Google AI Studio API key and Gemini model,
/// preferring runtime DB settings in `app_config` ("settings") over environment variables.
pub async fn resolve_gemini_settings(
    sea_db: &DatabaseConnection,
    env_api_key: Option<&str>,
    env_model: &str,
) -> (Option<String>, String) {
    let dynamic_settings = app_config::Entity::find_by_id("settings")
        .one(sea_db)
        .await
        .ok()
        .flatten();

    let mut api_key = env_api_key
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let mut model = env_model.trim().to_string();
    if model.is_empty() {
        model = "gemini-2.5-flash".to_string();
    }

    if let Some(cfg) = dynamic_settings {
        if let Some(v) = cfg.value.get("gemini_api_key").and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                api_key = Some(trimmed.to_string());
            }
        }
        if let Some(v) = cfg.value.get("gemini_model").and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                model = trimmed.to_string();
            }
        }
    }

    (api_key, model)
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<AppConfigDto>, AppError> {
    // Optionally read customized runtime overrides from SeaORM
    let dynamic_settings = app_config::Entity::find_by_id("settings")
        .one(&state.sea_db)
        .await
        .ok()
        .flatten();

    let mut dl = state.config.downloads_dir.to_string_lossy().to_string();
    let mut lib = state.config.library_dir.to_string_lossy().to_string();
    let mut cat = state.config.nyaa_default_category.clone();
    let mut dl_limit = state.config.torrent_global_download_limit_kbps;
    let mut ul_limit = state.config.torrent_global_upload_limit_kbps;
    let mut preset = state.config.encoder_default_preset.clone();
    let mut gemini_key = state.config.gemini_api_key.clone();
    let mut gemini_model = state.config.gemini_model.clone();

    if let Some(cfg) = dynamic_settings {
        if let Some(v) = cfg.value.get("downloads_dir").and_then(|v| v.as_str()) {
            dl = v.to_string();
        }
        if let Some(v) = cfg.value.get("library_dir").and_then(|v| v.as_str()) {
            lib = v.to_string();
        }
        if let Some(v) = cfg.value.get("nyaa_default_category").and_then(|v| v.as_str()) {
            cat = v.to_string();
        }
        if let Some(v) = cfg.value.get("torrent_global_download_limit_kbps").and_then(|v| v.as_u64()) {
            dl_limit = v;
        }
        if let Some(v) = cfg.value.get("torrent_global_upload_limit_kbps").and_then(|v| v.as_u64()) {
            ul_limit = v;
        }
        if let Some(v) = cfg.value.get("encoder_default_preset").and_then(|v| v.as_str()) {
            preset = v.to_string();
        }
        if let Some(v) = cfg.value.get("gemini_api_key").and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                gemini_key = Some(trimmed.to_string());
            }
        }
        if let Some(v) = cfg.value.get("gemini_model").and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                gemini_model = trimmed.to_string();
            }
        }
    }

    let gemini_api_key_configured = gemini_key.as_ref().is_some_and(|k| !k.trim().is_empty());
    let gemini_api_key_masked = gemini_key.as_deref().map(mask_api_key);

    let cfg = AppConfigDto {
        downloads_dir: dl,
        library_dir: lib,
        covers_dir: state.config.covers_dir.to_string_lossy().to_string(),
        temp_encode_dir: state.config.temp_encode_dir.to_string_lossy().to_string(),
        nyaa_base_url: state.config.nyaa_base_url.clone(),
        nyaa_default_category: cat,
        anilist_graphql_url: state.config.anilist_graphql_url.clone(),
        torrent_listen_port: state.config.torrent_listen_port,
        torrent_enable_dht: state.config.torrent_enable_dht,
        torrent_global_download_limit_kbps: dl_limit,
        torrent_global_upload_limit_kbps: ul_limit,
        ffmpeg_binary_path: state.config.ffmpeg_binary_path.clone(),
        ffprobe_binary_path: state.config.ffprobe_binary_path.clone(),
        encoder_default_preset: preset,
        gemini_api_key_configured,
        gemini_api_key_masked,
        gemini_model,
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

    let existing = app_config::Entity::find_by_id("settings")
        .one(&state.sea_db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query app_config via SeaORM: {e}")))?;

    let mut merged = existing
        .as_ref()
        .and_then(|c| c.value.as_object().cloned())
        .unwrap_or_default();

    if let Some(v) = payload.downloads_dir {
        merged.insert("downloads_dir".into(), json!(v));
    }
    if let Some(v) = payload.library_dir {
        merged.insert("library_dir".into(), json!(v));
    }
    if let Some(v) = payload.nyaa_default_category {
        merged.insert("nyaa_default_category".into(), json!(v));
    }
    if let Some(v) = payload.torrent_global_download_limit_kbps {
        merged.insert("torrent_global_download_limit_kbps".into(), json!(v));
    }
    if let Some(v) = payload.torrent_global_upload_limit_kbps {
        merged.insert("torrent_global_upload_limit_kbps".into(), json!(v));
    }
    if let Some(v) = payload.encoder_default_preset {
        merged.insert("encoder_default_preset".into(), json!(v));
    }
    if payload.clear_gemini_api_key.unwrap_or(false) {
        merged.remove("gemini_api_key");
    } else if let Some(key) = payload.gemini_api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            merged.insert("gemini_api_key".into(), json!(trimmed));
        }
    }
    if let Some(model) = payload.gemini_model {
        let trimmed = model.trim();
        if !trimmed.is_empty() {
            merged.insert("gemini_model".into(), json!(trimmed));
        }
    }

    let config_val = serde_json::Value::Object(merged);

    if let Some(cfg) = existing {
        let mut active: app_config::ActiveModel = cfg.into();
        active.value = Set(config_val);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&state.sea_db).await.map_err(|e| {
            AppError::Internal(format!("Failed to update config via SeaORM: {e}"))
        })?;
    } else {
        let new_cfg = app_config::ActiveModel {
            key: Set("settings".to_string()),
            value: Set(config_val),
            updated_at: Set(chrono::Utc::now()),
        };
        new_cfg.insert(&state.sea_db).await.map_err(|e| {
            AppError::Internal(format!("Failed to insert config via SeaORM: {e}"))
        })?;
    }

    Ok(Json(json!({
        "status": "success",
        "message": "Configuration updated successfully via SeaORM"
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
