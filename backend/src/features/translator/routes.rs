use std::path::Path;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::auth::AdminUser;
use crate::features::config::routes::resolve_gemini_settings;
use crate::features::encoder::ffmpeg_runner::{
    check_ffmpeg_available, check_ffprobe_available, resolve_binary_path,
};
use crate::features::translator::ffmpeg_sub::{pick_recommended_track, probe_subtitle_tracks};
use crate::features::translator::gemini_client::GeminiClient;
use crate::features::translator::models::{
    infer_language_code, ProbeSubtitlesRequest, ProbeSubtitlesResponse, QueueTranslationRequest,
    TestGeminiKeyRequest, TranslationJob, TranslationJobItem, TranslationJobsFilterQuery,
    TranslatorStatusResponse,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_status))
        .route("/test-key", post(test_gemini_key))
        .route("/probe", post(probe_subtitles))
        .route("/jobs", get(list_jobs).post(queue_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/jobs/:id", delete(delete_job))
        .route("/jobs/:id/subtitle", get(download_translated_subtitle))
}

async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<TranslatorStatusResponse>, AppError> {
    let (api_key, default_model) = resolve_gemini_settings(
        &state.sea_db,
        state.config.gemini_api_key.as_deref(),
        &state.config.gemini_model,
    )
    .await;

    let resolved_ffmpeg = resolve_binary_path(&state.config.ffmpeg_binary_path);
    let resolved_ffprobe = resolve_binary_path(&state.config.ffprobe_binary_path);
    let ffmpeg_available = check_ffmpeg_available(&resolved_ffmpeg).await.is_ok();
    let ffprobe_available = check_ffprobe_available(&resolved_ffprobe).await.is_ok();

    Ok(Json(TranslatorStatusResponse {
        gemini_configured: api_key.is_some(),
        default_model,
        ffmpeg_available,
        ffprobe_available,
    }))
}

async fn test_gemini_key(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<TestGeminiKeyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (stored_key, default_model) = resolve_gemini_settings(
        &state.sea_db,
        state.config.gemini_api_key.as_deref(),
        &state.config.gemini_model,
    )
    .await;

    let key = payload
        .api_key
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or(stored_key)
        .ok_or_else(|| AppError::BadRequest("No Google AI Studio API key provided or saved".into()))?;

    let model = payload
        .model
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_model);

    let client = GeminiClient::new();
    let display_name = client.test_api_key(&key, &model).await?;

    Ok(Json(json!({
        "status": "success",
        "valid": true,
        "model": model,
        "display_name": display_name,
        "message": format!("Connected to Google AI Studio ({display_name})")
    })))
}

async fn probe_subtitles(
    State(state): State<AppState>,
    Json(payload): Json<ProbeSubtitlesRequest>,
) -> Result<Json<ProbeSubtitlesResponse>, AppError> {
    let tracks = probe_subtitle_tracks(&state.config.ffprobe_binary_path, &payload.file_path).await?;
    let recommended = pick_recommended_track(&tracks).map(|t| t.stream_index);

    let mut warnings = Vec::new();
    if tracks.is_empty() {
        warnings.push("No embedded subtitle tracks found in this video file.".into());
    } else if recommended.is_none() {
        warnings.push(
            "Only bitmap/image-based subtitles (e.g. PGS/VobSub) were found; text extraction requires ASS/SRT/WebVTT tracks.".into(),
        );
    }

    Ok(Json(ProbeSubtitlesResponse {
        file_path: payload.file_path,
        tracks,
        recommended_stream_index: recommended,
        warnings,
    }))
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<TranslationJobsFilterQuery>,
) -> Result<Json<Vec<TranslationJobItem>>, AppError> {
    let jobs = if let Some(status) = query.status.filter(|s| !s.is_empty() && s != "ALL") {
        if status == "RUNNING" {
            sqlx::query_as::<_, TranslationJob>(
                r#"
                SELECT
                    id, library_file_id, source_file_path, target_file_path,
                    extracted_sub_path, translated_sub_path, source_stream_index,
                    source_sub_codec, source_language, target_language,
                    target_language_code, gemini_model, embed_mode, status,
                    progress_pct, total_cues, translated_cues, current_batch,
                    total_batches, stage_message, error_message,
                    created_at, started_at, completed_at
                FROM translation_jobs
                WHERE status IN ('EXTRACTING', 'TRANSLATING', 'EMBEDDING')
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, TranslationJob>(
                r#"
                SELECT
                    id, library_file_id, source_file_path, target_file_path,
                    extracted_sub_path, translated_sub_path, source_stream_index,
                    source_sub_codec, source_language, target_language,
                    target_language_code, gemini_model, embed_mode, status,
                    progress_pct, total_cues, translated_cues, current_batch,
                    total_batches, stage_message, error_message,
                    created_at, started_at, completed_at
                FROM translation_jobs
                WHERE status = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(status)
            .fetch_all(&state.db)
            .await?
        }
    } else {
        sqlx::query_as::<_, TranslationJob>(
            r#"
            SELECT
                id, library_file_id, source_file_path, target_file_path,
                extracted_sub_path, translated_sub_path, source_stream_index,
                source_sub_codec, source_language, target_language,
                target_language_code, gemini_model, embed_mode, status,
                progress_pct, total_cues, translated_cues, current_batch,
                total_batches, stage_message, error_message,
                created_at, started_at, completed_at
            FROM translation_jobs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&state.db)
        .await?
    };

    let items = jobs
        .into_iter()
        .map(|job| {
            let source_filename = Path::new(&job.source_file_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "video.mkv".into());
            let target_filename = Path::new(&job.target_file_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "output.mkv".into());
            let has_translated_sub = job
                .translated_sub_path
                .as_deref()
                .is_some_and(|p| Path::new(p).is_file());

            TranslationJobItem {
                job,
                source_filename,
                target_filename,
                has_translated_sub,
            }
        })
        .collect();

    Ok(Json(items))
}

pub async fn queue_translation_internal(
    state: &AppState,
    payload: QueueTranslationRequest,
) -> Result<TranslationJob, AppError> {
    let resolved_ffmpeg = resolve_binary_path(&state.config.ffmpeg_binary_path);
    if let Err(err) = check_ffmpeg_available(&resolved_ffmpeg).await {
        return Err(AppError::BadRequest(format!(
            "FFmpeg executable not found (path: '{resolved_ffmpeg}'). Error: {err}."
        )));
    }

    let (api_key_opt, default_model) = resolve_gemini_settings(
        &state.sea_db,
        state.config.gemini_api_key.as_deref(),
        &state.config.gemini_model,
    )
    .await;

    if api_key_opt.is_none() {
        return Err(AppError::BadRequest(
            "Google AI Studio API key is not configured. Configure it in System Settings before queueing translations.".into(),
        ));
    }

    let source_path = Path::new(&payload.source_file_path);
    if !source_path.is_file() {
        return Err(AppError::NotFound(format!(
            "Source video file does not exist: {}",
            payload.source_file_path
        )));
    }

    let target_language = payload.target_language.trim().to_string();
    if target_language.is_empty() {
        return Err(AppError::BadRequest("Target language is required".into()));
    }

    let target_language_code = payload
        .target_language_code
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| infer_language_code(&target_language));

    let gemini_model = payload
        .gemini_model
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_model);

    let embed_mode = match payload.embed_mode.as_deref() {
        Some("mux_replace") => "mux_replace".to_string(),
        _ => "mux_keep_original".to_string(),
    };

    // Probe upfront if no stream index specified, or validate requested stream
    let tracks = probe_subtitle_tracks(&state.config.ffprobe_binary_path, &payload.source_file_path).await?;
    if tracks.is_empty() {
        return Err(AppError::BadRequest(
            "No embedded subtitle tracks found in source video file.".into(),
        ));
    }

    let selected_track = match payload.source_stream_index {
        Some(idx) => tracks
            .iter()
            .find(|t| t.stream_index == idx)
            .cloned()
            .ok_or_else(|| {
                AppError::BadRequest(format!("Subtitle stream #{idx} not found in source video."))
            })?,
        None => pick_recommended_track(&tracks).cloned().ok_or_else(|| {
            AppError::BadRequest(
                "No text-based subtitle tracks found in source video (only image-based PGS/VobSub tracks exist).".into(),
            )
        })?,
    };

    if !selected_track.is_text_based {
        return Err(AppError::BadRequest(format!(
            "Selected subtitle stream #{} ({}) is image-based and cannot be translated as text.",
            selected_track.stream_index, selected_track.codec_name
        )));
    }

    let target_file_path = match payload.target_file_path.filter(|s| !s.trim().is_empty()) {
        Some(target) => target.trim().to_string(),
        None => {
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video");
            let ext = source_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("mkv");
            let parent = source_path.parent().unwrap_or(&state.config.temp_encode_dir);
            let filename = format!("{stem} [{target_language_code}].{ext}");
            parent.join(filename).to_string_lossy().to_string()
        }
    };

    let source_language = payload
        .source_language
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or(selected_track.language.clone());

    let job = sqlx::query_as::<_, TranslationJob>(
        r#"
        INSERT INTO translation_jobs (
            library_file_id, source_file_path, target_file_path,
            source_stream_index, source_sub_codec, source_language,
            target_language, target_language_code, gemini_model,
            embed_mode, status, stage_message
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'QUEUED', 'Waiting in queue...')
        RETURNING
            id, library_file_id, source_file_path, target_file_path,
            extracted_sub_path, translated_sub_path, source_stream_index,
            source_sub_codec, source_language, target_language,
            target_language_code, gemini_model, embed_mode, status,
            progress_pct, total_cues, translated_cues, current_batch,
            total_batches, stage_message, error_message,
            created_at, started_at, completed_at
        "#,
    )
    .bind(payload.library_file_id)
    .bind(payload.source_file_path)
    .bind(target_file_path)
    .bind(selected_track.stream_index)
    .bind(selected_track.codec_name)
    .bind(source_language)
    .bind(target_language)
    .bind(target_language_code)
    .bind(gemini_model)
    .bind(embed_mode)
    .fetch_one(&state.db)
    .await?;

    Ok(job)
}

async fn queue_job(
    State(state): State<AppState>,
    Json(payload): Json<QueueTranslationRequest>,
) -> Result<Json<TranslationJob>, AppError> {
    let job = queue_translation_internal(&state, payload).await?;
    Ok(Json(job))
}

async fn cancel_job(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query(
        "UPDATE translation_jobs SET status = 'CANCELLED', stage_message = 'Cancelled by user', completed_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Translation job cancelled",
        "id": id
    })))
}

async fn delete_job(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let row = sqlx::query_as::<_, TranslationJob>(
        r#"
        SELECT
            id, library_file_id, source_file_path, target_file_path,
            extracted_sub_path, translated_sub_path, source_stream_index,
            source_sub_codec, source_language, target_language,
            target_language_code, gemini_model, embed_mode, status,
            progress_pct, total_cues, translated_cues, current_batch,
            total_batches, stage_message, error_message,
            created_at, started_at, completed_at
        FROM translation_jobs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(job) = row {
        if let Some(p) = job.extracted_sub_path {
            let _ = tokio::fs::remove_file(p).await;
        }
        if let Some(p) = job.translated_sub_path {
            let _ = tokio::fs::remove_file(p).await;
        }
    }

    sqlx::query("DELETE FROM translation_jobs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Translation job deleted",
        "id": id
    })))
}

async fn download_translated_subtitle(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let job = sqlx::query_as::<_, TranslationJob>(
        r#"
        SELECT
            id, library_file_id, source_file_path, target_file_path,
            extracted_sub_path, translated_sub_path, source_stream_index,
            source_sub_codec, source_language, target_language,
            target_language_code, gemini_model, embed_mode, status,
            progress_pct, total_cues, translated_cues, current_batch,
            total_batches, stage_message, error_message,
            created_at, started_at, completed_at
        FROM translation_jobs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Translation job {id} not found")))?;

    let sub_path = job.translated_sub_path.ok_or_else(|| {
        AppError::NotFound("Translated subtitle file is not yet available".into())
    })?;

    let content = tokio::fs::read(&sub_path).await.map_err(|e| {
        AppError::NotFound(format!("Translated subtitle file not found on disk: {e}"))
    })?;

    let ext = Path::new(&sub_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("srt");
    let stem = Path::new(&job.source_file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("subtitle");
    let download_name = format!("{stem}.{}.{ext}", job.target_language_code);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{download_name}\""),
            ),
        ],
        content,
    ))
}
