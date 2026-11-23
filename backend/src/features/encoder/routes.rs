use std::path::Path;
use axum::{
    extract::{Path as AxumPath, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::encoder::models::{
    CreatePresetRequest, EncodingJob, EncodingJobItem, EncodingPreset, JobsFilterQuery,
    QueueJobRequest,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/presets", get(list_presets).post(create_preset))
        .route("/jobs", get(list_jobs).post(queue_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/jobs/:id", delete(delete_job))
}

async fn list_presets(
    State(state): State<AppState>,
) -> Result<Json<Vec<EncodingPreset>>, AppError> {
    let mut presets = sqlx::query_as::<_, EncodingPreset>(
        r#"
        SELECT
            id, name, description, video_codec, preset, crf,
            target_resolution, audio_codec, audio_bitrate, container,
            custom_ffmpeg_args, is_default, created_at
        FROM encoding_presets
        ORDER BY is_default DESC, name ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    // Seed defaults if empty
    if presets.is_empty() {
        let _ = sqlx::query(
            r#"
            INSERT INTO encoding_presets (name, description, video_codec, preset, crf, target_resolution, audio_codec, container, is_default)
            VALUES
            ('720p x265 Balanced', 'Compress to 720p with HEVC CRF 24 medium preset', 'libx265', 'medium', 24, '1280x720', 'copy', 'mkv', TRUE),
            ('1080p x265 High Quality', 'Re-encode to 1080p HEVC CRF 21 for storage efficiency', 'libx265', 'medium', 21, '1920x1080', 'copy', 'mkv', FALSE)
            ON CONFLICT (name) DO NOTHING
            "#
        )
        .execute(&state.db)
        .await;

        presets = sqlx::query_as::<_, EncodingPreset>(
            r#"
            SELECT
                id, name, description, video_codec, preset, crf,
                target_resolution, audio_codec, audio_bitrate, container,
                custom_ffmpeg_args, is_default, created_at
            FROM encoding_presets
            ORDER BY is_default DESC, name ASC
            "#
        )
        .fetch_all(&state.db)
        .await?;
    }

    Ok(Json(presets))
}

async fn create_preset(
    State(state): State<AppState>,
    Json(payload): Json<CreatePresetRequest>,
) -> Result<Json<EncodingPreset>, AppError> {
    let video_codec = payload.video_codec.unwrap_or_else(|| "libx265".into());
    let preset = payload.preset.unwrap_or_else(|| "medium".into());
    let crf = payload.crf.unwrap_or(24);
    let audio_codec = payload.audio_codec.unwrap_or_else(|| "copy".into());
    let audio_bitrate = payload.audio_bitrate.or_else(|| Some("160k".into()));
    let container = payload.container.unwrap_or_else(|| "mkv".into());
    let is_default = payload.is_default.unwrap_or(false);

    if is_default {
        let _ = sqlx::query("UPDATE encoding_presets SET is_default = FALSE WHERE is_default = TRUE")
            .execute(&state.db)
            .await;
    }

    let created = sqlx::query_as::<_, EncodingPreset>(
        r#"
        INSERT INTO encoding_presets (
            name, description, video_codec, preset, crf, target_resolution,
            audio_codec, audio_bitrate, container, custom_ffmpeg_args, is_default
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING
            id, name, description, video_codec, preset, crf, target_resolution,
            audio_codec, audio_bitrate, container, custom_ffmpeg_args, is_default, created_at
        "#
    )
    .bind(payload.name)
    .bind(payload.description)
    .bind(video_codec)
    .bind(preset)
    .bind(crf)
    .bind(payload.target_resolution)
    .bind(audio_codec)
    .bind(audio_bitrate)
    .bind(container)
    .bind(payload.custom_ffmpeg_args)
    .bind(is_default)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(created))
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<JobsFilterQuery>,
) -> Result<Json<Vec<EncodingJobItem>>, AppError> {
    let mut sql = String::from(
        r#"
        SELECT
            ej.id, ej.source_file_path, ej.target_file_path, ej.preset_id, ej.status,
            ej.progress_pct, ej.current_frame, ej.total_frames, ej.fps, ej.current_time_sec,
            ej.total_duration_sec, ej.speed, ej.source_size_bytes, ej.encoded_size_bytes,
            ej.eta_seconds, ej.error_message, ej.created_at, ej.started_at, ej.completed_at,
            ep.name AS preset_name
        FROM encoding_jobs ej
        LEFT JOIN encoding_presets ep ON ej.preset_id = ep.id
        WHERE 1=1
        "#,
    );

    if let Some(status) = query.status {
        if !status.is_empty() && status != "ALL" {
            sql.push_str(&format!(" AND ej.status = '{}'", status.replace('\'', "''")));
        }
    }

    sql.push_str(" ORDER BY ej.created_at DESC");

    let rows = sqlx::query(&sql).fetch_all(&state.db).await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let job = EncodingJob {
                id: row.get("id"),
                source_file_path: row.get("source_file_path"),
                target_file_path: row.get("target_file_path"),
                preset_id: row.get("preset_id"),
                status: row.get("status"),
                progress_pct: row.get("progress_pct"),
                current_frame: row.get("current_frame"),
                total_frames: row.get("total_frames"),
                fps: row.get("fps"),
                current_time_sec: row.get("current_time_sec"),
                total_duration_sec: row.get("total_duration_sec"),
                speed: row.get("speed"),
                source_size_bytes: row.get("source_size_bytes"),
                encoded_size_bytes: row.get("encoded_size_bytes"),
                eta_seconds: row.get("eta_seconds"),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
            };
            let preset_name: String = row.try_get("preset_name").unwrap_or_else(|_| "Unknown Preset".into());
            let source_filename = Path::new(&job.source_file_path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "video.mkv".into());

            EncodingJobItem {
                job,
                preset_name,
                source_filename,
            }
        })
        .collect();

    Ok(Json(items))
}

pub async fn queue_job_internal(
    state: &AppState,
    payload: QueueJobRequest,
) -> Result<EncodingJob, AppError> {
    if !Path::new(&payload.source_file_path).exists() {
        return Err(AppError::NotFound(format!(
            "Source file does not exist: {}",
            payload.source_file_path
        )));
    }

    // Resolve preset_id
    let preset_id = match payload.preset_id {
        Some(id) => id,
        None => {
            let default_preset = sqlx::query("SELECT id FROM encoding_presets WHERE is_default = TRUE LIMIT 1")
                .fetch_optional(&state.db)
                .await?
                .or_else(|| None);

            match default_preset {
                Some(row) => row.get("id"),
                None => {
                    let first_preset = sqlx::query("SELECT id FROM encoding_presets ORDER BY created_at ASC LIMIT 1")
                        .fetch_optional(&state.db)
                        .await?
                        .ok_or_else(|| AppError::BadRequest("No encoding presets configured".into()))?;
                    first_preset.get("id")
                }
            }
        }
    };

    // Resolve target_file_path
    let target_file_path = match payload.target_file_path {
        Some(target) => target,
        None => {
            let source_path = Path::new(&payload.source_file_path);
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("video");
            let target_filename = format!("{stem}_encoded.mkv");
            state.config.temp_encode_dir.join(target_filename).to_string_lossy().to_string()
        }
    };

    let source_size = tokio::fs::metadata(&payload.source_file_path)
        .await
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let job = sqlx::query_as::<_, EncodingJob>(
        r#"
        INSERT INTO encoding_jobs (
            source_file_path, target_file_path, preset_id, status,
            progress_pct, current_frame, fps, current_time_sec,
            total_duration_sec, speed, source_size_bytes, encoded_size_bytes
        ) VALUES ($1, $2, $3, 'QUEUED', 0.0, 0, 0.0, 0.0, 0.0, 0.0, $4, 0)
        RETURNING
            id, source_file_path, target_file_path, preset_id, status,
            progress_pct, current_frame, total_frames, fps, current_time_sec,
            total_duration_sec, speed, source_size_bytes, encoded_size_bytes,
            eta_seconds, error_message, created_at, started_at, completed_at
        "#
    )
    .bind(payload.source_file_path)
    .bind(target_file_path)
    .bind(preset_id)
    .bind(source_size)
    .fetch_one(&state.db)
    .await?;

    Ok(job)
}

async fn queue_job(
    State(state): State<AppState>,
    Json(payload): Json<QueueJobRequest>,
) -> Result<Json<EncodingJob>, AppError> {
    let job = queue_job_internal(&state, payload).await?;
    Ok(Json(job))
}

async fn cancel_job(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("UPDATE encoding_jobs SET status = 'CANCELLED' WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Encoding job cancelled",
        "id": id
    })))
}

async fn delete_job(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM encoding_jobs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Encoding job deleted",
        "id": id
    })))
}
