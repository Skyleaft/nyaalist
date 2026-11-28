use std::path::PathBuf;
use axum::{
    extract::{Path as AxumPath, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::Row;
use tracing::info;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::encoder::models::{EncodingJob, QueueJobRequest};
use crate::features::encoder::routes::queue_job_internal;
use crate::features::library::models::{
    AutoOrganizeRequest, EncodeFileRequest, LibraryFile, LibraryFileItem, LibraryFilesQuery, MoveFileRequest,
    ScanDirectoryResult, TranslateFileRequest,
};
use crate::features::library::organizer::{
    auto_organize_file, move_or_copy_file, scan_and_sync_downloads,
};
use crate::features::translator::models::{QueueTranslationRequest, TranslationJob};
use crate::features::translator::routes::queue_translation_internal;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", get(get_library_files))
        .route("/scan", post(scan_library_downloads))
        .route("/move", post(manual_move_file))
        .route("/auto-organize", post(auto_organize_endpoint))
        .route("/files/:id", delete(delete_library_file))
        .route("/files/:id/encode", post(encode_library_file))
        .route("/files/:id/translate", post(translate_library_file))
}

async fn get_library_files(
    State(state): State<AppState>,
    Query(query): Query<LibraryFilesQuery>,
) -> Result<Json<Vec<LibraryFileItem>>, AppError> {
    let mut sql = String::from(
        r#"
        SELECT
            lf.id, lf.download_job_id, lf.anilist_id, lf.original_filename, lf.current_path,
            lf.target_library_path, lf.episode, lf.season, lf.resolution, lf.file_size_bytes,
            lf.is_moved, lf.is_encoded, lf.created_at, lf.updated_at,
            COALESCE(am.title_english, am.title_romaji) AS anime_title,
            am.cover_image_local_path AS anime_cover_path
        FROM library_files lf
        LEFT JOIN anime_metadata am ON lf.anilist_id = am.id
        WHERE 1=1
        "#,
    );

    if let Some(is_moved) = query.is_moved {
        if is_moved {
            sql.push_str(" AND lf.is_moved = TRUE");
        } else {
            sql.push_str(" AND lf.is_moved = FALSE");
        }
    }

    if let Some(search) = &query.search {
        if !search.trim().is_empty() {
            sql.push_str(&format!(
                " AND (lf.original_filename ILIKE '%{}%' OR am.title_romaji ILIKE '%{}%' OR am.title_english ILIKE '%{}%')",
                search.replace('\'', "''"),
                search.replace('\'', "''"),
                search.replace('\'', "''")
            ));
        }
    }

    sql.push_str(" ORDER BY lf.created_at DESC");

    let rows = sqlx::query(&sql).fetch_all(&state.db).await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let file = LibraryFile {
                id: row.get("id"),
                download_job_id: row.get("download_job_id"),
                anilist_id: row.get("anilist_id"),
                original_filename: row.get("original_filename"),
                current_path: row.get("current_path"),
                target_library_path: row.get("target_library_path"),
                episode: row.get("episode"),
                season: row.get("season"),
                resolution: row.get("resolution"),
                file_size_bytes: row.get("file_size_bytes"),
                is_moved: row.get("is_moved"),
                is_encoded: row.get("is_encoded"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            let anime_title: Option<String> = row.get("anime_title");
            let anime_cover_path: Option<String> = row.get("anime_cover_path");

            LibraryFileItem {
                file,
                anime_title,
                anime_cover_path,
            }
        })
        .collect();

    Ok(Json(items))
}

async fn scan_library_downloads(
    State(state): State<AppState>,
) -> Result<Json<ScanDirectoryResult>, AppError> {
    let result = scan_and_sync_downloads(
        &state.config.downloads_dir,
        &state.config.library_dir,
        &state.db,
    )
    .await?;

    Ok(Json(result))
}

async fn manual_move_file(
    State(state): State<AppState>,
    Json(payload): Json<MoveFileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let source = PathBuf::from(&payload.source_path);
    let mut target = PathBuf::from(&payload.target_path);

    if target.is_dir() || target.extension().is_none() {
        let filename = payload.new_filename.as_deref().unwrap_or_else(|| {
            source
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("media.mkv")
        });
        target = target.join(filename);
    }

    move_or_copy_file(&source, &target).await?;

    let target_str = target.to_string_lossy().to_string();

    if let Some(file_id) = payload.file_id {
        sqlx::query(
            "UPDATE library_files SET current_path = $1, target_library_path = $1, is_moved = TRUE, updated_at = NOW() WHERE id = $2"
        )
        .bind(&target_str)
        .bind(file_id)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({
        "status": "success",
        "source": payload.source_path,
        "target": target_str
    })))
}

async fn auto_organize_endpoint(
    State(state): State<AppState>,
    Json(payload): Json<AutoOrganizeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(file_id) = payload.file_id {
        let organized = auto_organize_file(
            file_id,
            payload.template,
            &state.config.library_dir,
            &state.db,
        )
        .await?;

        return Ok(Json(json!({
            "status": "success",
            "organized_count": 1,
            "files": [organized]
        })));
    }

    // Batch organize all unorganized files
    let unorganized = sqlx::query_as::<_, LibraryFile>(
        "SELECT * FROM library_files WHERE is_moved = FALSE ORDER BY created_at ASC"
    )
    .fetch_all(&state.db)
    .await?;

    let mut organized_files = Vec::new();
    for file in unorganized {
        match auto_organize_file(
            file.id,
            payload.template.clone(),
            &state.config.library_dir,
            &state.db,
        )
        .await
        {
            Ok(org) => organized_files.push(org),
            Err(err) => {
                info!("Skipped file {}: {}", file.original_filename, err);
            }
        }
    }

    Ok(Json(json!({
        "status": "success",
        "organized_count": organized_files.len(),
        "files": organized_files
    })))
}

async fn delete_library_file(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM library_files WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Library file removed from tracking",
        "id": id
    })))
}

async fn encode_library_file(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
    Json(payload): Json<EncodeFileRequest>,
) -> Result<Json<EncodingJob>, AppError> {
    let file = sqlx::query_as::<_, LibraryFile>(
        r#"
        SELECT
            id, download_job_id, anilist_id, original_filename, current_path,
            target_library_path, episode, season, resolution, file_size_bytes,
            is_moved, is_encoded, created_at, updated_at
        FROM library_files
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Library file not found with ID {id}")))?;

    let queue_payload = QueueJobRequest {
        source_file_path: file.current_path,
        preset_id: payload.preset_id,
        target_file_path: payload.target_file_path,
    };

    let job = queue_job_internal(&state, queue_payload).await?;
    info!("Queued encode job {} for library file {}", job.id, file.id);
    Ok(Json(job))
}

async fn translate_library_file(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<Uuid>,
    Json(payload): Json<TranslateFileRequest>,
) -> Result<Json<TranslationJob>, AppError> {
    let file = sqlx::query_as::<_, LibraryFile>(
        r#"
        SELECT
            id, download_job_id, anilist_id, original_filename, current_path,
            target_library_path, episode, season, resolution, file_size_bytes,
            is_moved, is_encoded, created_at, updated_at
        FROM library_files
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Library file not found with ID {id}")))?;

    let queue_payload = QueueTranslationRequest {
        library_file_id: Some(file.id),
        source_file_path: file.current_path,
        target_file_path: payload.target_file_path,
        source_stream_index: payload.source_stream_index,
        source_language: payload.source_language,
        target_language: payload.target_language,
        target_language_code: payload.target_language_code,
        gemini_model: payload.gemini_model,
        embed_mode: payload.embed_mode,
    };

    let job = queue_translation_internal(&state, queue_payload).await?;
    info!("Queued AI translation job {} for library file {}", job.id, file.id);
    Ok(Json(job))
}
