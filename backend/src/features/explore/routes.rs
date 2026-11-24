use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::explore::models::{
    AnimeMetadata, ExploreQuery, NyaaRelease, NyaaReleaseDetail, SyncRequest,
};
use crate::features::explore::nyaa_scraper::{
    scrape_nyaa_category, scrape_nyaa_user, scrape_nyaa_view,
};
use crate::features::explore::title_parser::parse_release_title;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/latest", get(get_latest_releases))
        .route("/user/:username", get(get_user_releases))
        .route("/view/:id", get(get_release_detail))
        .route("/detail/:id", get(get_release_detail))
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
    let sort = query.sort.as_deref();
    let order = query.order.as_deref();

    let mut releases = scrape_nyaa_category(
        &state.config.nyaa_base_url,
        category,
        page,
        search,
        sort,
        order,
        &state.config.nyaa_user_agent,
    )
    .await?;

    // Collect all parsed anime titles from scraped releases
    let titles: Vec<String> = releases
        .iter()
        .map(|r| r.parsed.anime_title.clone())
        .filter(|t| !t.is_empty())
        .collect();

    // High-performance batch search: check local DB first (title_mappings & anime_metadata),
    // and only if not found in local DB does it queue to AniList API in the background.
    let metadata_map = state
        .anilist_client
        .resolve_titles_batch(&state.db, &titles)
        .await;

    // Attach resolved metadata in-memory
    for rel in &mut releases {
        if let Some(meta) = metadata_map.get(&rel.parsed.anime_title) {
            rel.anilist_metadata = Some(meta.clone());
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
    let search = query.query.as_deref();
    let sort = query.sort.as_deref();
    let order = query.order.as_deref();

    let mut releases = scrape_nyaa_user(
        &state.config.nyaa_base_url,
        &username,
        page,
        search,
        sort,
        order,
        &state.config.nyaa_user_agent,
    )
    .await?;

    let titles: Vec<String> = releases
        .iter()
        .map(|r| r.parsed.anime_title.clone())
        .filter(|t| !t.is_empty())
        .collect();

    let metadata_map = state
        .anilist_client
        .resolve_titles_batch(&state.db, &titles)
        .await;

    for rel in &mut releases {
        if let Some(meta) = metadata_map.get(&rel.parsed.anime_title) {
            rel.anilist_metadata = Some(meta.clone());
        }
    }

    Ok(Json(releases))
}

async fn get_release_detail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NyaaReleaseDetail>, AppError> {
    let mut detail = scrape_nyaa_view(
        &state.config.nyaa_base_url,
        id,
        &state.config.nyaa_user_agent,
    )
    .await?;

    if !detail.parsed.anime_title.is_empty() {
        let metadata_map = state
            .anilist_client
            .resolve_titles_batch(&state.db, &[detail.parsed.anime_title.clone()])
            .await;

        if let Some(meta) = metadata_map.get(&detail.parsed.anime_title) {
            detail.anilist_metadata = Some(meta.clone());
        }
    }

    Ok(Json(detail))
}

async fn sync_title_metadata(
    State(state): State<AppState>,
    Json(payload): Json<SyncRequest>,
) -> Result<Json<Option<AnimeMetadata>>, AppError> {
    let raw = payload.title.trim();
    let parsed = parse_release_title(raw);
    let title_to_sync = if !parsed.anime_title.is_empty() {
        &parsed.anime_title
    } else {
        raw
    };

    let meta = state
        .anilist_client
        .sync_anime_metadata(&state.db, title_to_sync)
        .await?;

    Ok(Json(meta))
}

async fn serve_cover_image(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let clean_filename = filename.replace("..", "").replace(['/', '\\'], "");
    let file_path = state.config.covers_dir.join(&clean_filename);

    let bytes = if file_path.exists() {
        tokio::fs::read(&file_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read cover image: {}", e)))?
    } else {
        tracing::info!(
            "Cover image '{}' not found on disk, redownloading from source...",
            clean_filename
        );
        state
            .anilist_client
            .redownload_cover(&state.db, &clean_filename)
            .await?
    };

    let content_type = if clean_filename.ends_with(".png") {
        "image/png"
    } else if clean_filename.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=86400".parse().unwrap(),
    );

    Ok((StatusCode::OK, headers, bytes))
}
