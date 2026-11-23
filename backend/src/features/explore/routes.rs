use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::explore::models::{AnimeMetadata, ExploreQuery, NyaaRelease, SyncRequest};
use crate::features::explore::nyaa_scraper::{scrape_nyaa_category, scrape_nyaa_user};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/latest", get(get_latest_releases))
        .route("/user/:username", get(get_user_releases))
        .route("/sync", post(sync_title_metadata))
}

pub fn covers_router() -> Router<AppState> {
    Router::new().route("/:filename", get(serve_cover_image))
}

async fn get_latest_releases(
    State(state): State<AppState>,
    Query(query): Query<ExploreQuery>,
) -> Result<Json<Vec<NyaaRelease>>, AppError> {
    let category = query
        .category
        .as_deref()
        .unwrap_or(&state.config.nyaa_default_category);
    let page = query.page.unwrap_or(1);
    let search = query.query.as_deref();

    let mut releases = scrape_nyaa_category(
        &state.config.nyaa_base_url,
        category,
        page,
        search,
        &state.config.nyaa_user_agent,
    )
    .await?;

    // Check cached metadata locally (fast response without blocking on remote API);
    // Uncached titles are queued asynchronously into the rate-limited background worker
    for rel in &mut releases {
        if !rel.parsed.anime_title.is_empty() {
            if let Ok(Some(cached)) = state
                .anilist_client
                .get_cached_metadata(&state.db, &rel.parsed.anime_title)
                .await
            {
                rel.anilist_metadata = Some(cached);
            } else if let Ok(false) = state
                .anilist_client
                .is_title_checked(&state.db, &rel.parsed.anime_title)
                .await
            {
                state
                    .anilist_client
                    .enqueue_metadata_sync(&rel.parsed.anime_title)
                    .await;
            }
        }
    }

    Ok(Json(releases))
}

async fn get_user_releases(
    State(state): State<AppState>,
    Path(username): Path<String>,
    Query(query): Query<ExploreQuery>,
) -> Result<Json<Vec<NyaaRelease>>, AppError> {
    let page = query.page.unwrap_or(1);

    let mut releases = scrape_nyaa_user(
        &state.config.nyaa_base_url,
        &username,
        page,
        &state.config.nyaa_user_agent,
    )
    .await?;

    for rel in &mut releases {
        if !rel.parsed.anime_title.is_empty() {
            if let Ok(Some(cached)) = state
                .anilist_client
                .get_cached_metadata(&state.db, &rel.parsed.anime_title)
                .await
            {
                rel.anilist_metadata = Some(cached);
            } else if let Ok(false) = state
                .anilist_client
                .is_title_checked(&state.db, &rel.parsed.anime_title)
                .await
            {
                state
                    .anilist_client
                    .enqueue_metadata_sync(&rel.parsed.anime_title)
                    .await;
            }
        }
    }

    Ok(Json(releases))
}

async fn sync_title_metadata(
    State(state): State<AppState>,
    Json(payload): Json<SyncRequest>,
) -> Result<Json<Option<AnimeMetadata>>, AppError> {
    let meta = state
        .anilist_client
        .sync_anime_metadata(&state.db, &payload.title)
        .await?;

    Ok(Json(meta))
}

async fn serve_cover_image(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let clean_filename = filename.replace("..", "").replace(['/', '\\'], "");
    let file_path = state.config.covers_dir.join(&clean_filename);

    if !file_path.exists() {
        return Err(AppError::NotFound("Cover image not found".into()));
    }

    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read cover image: {}", e)))?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=86400".parse().unwrap(),
    );

    Ok((StatusCode::OK, headers, bytes))
}
