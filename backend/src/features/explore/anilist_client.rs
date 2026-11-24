use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration, Instant};

use crate::core::error::AppError;
use crate::core::ws::WsHub;
use crate::features::explore::models::AnimeMetadata;

struct RateLimiter {
    last_request: Instant,
    retry_after_until: Option<Instant>,
    min_interval: Duration,
}

#[derive(Debug, sqlx::FromRow)]
struct MappedMetadataRow {
    mapping_key: String,
    id: i32,
    mal_id: Option<i32>,
    title_romaji: String,
    title_english: Option<String>,
    title_native: Option<String>,
    format: Option<String>,
    status: Option<String>,
    episodes: Option<i32>,
    season: Option<String>,
    season_year: Option<i32>,
    genres: Option<Vec<String>>,
    average_score: Option<i32>,
    description: Option<String>,
    cover_image_url: Option<String>,
    cover_image_local_path: Option<String>,
    banner_image_url: Option<String>,
    last_synced_at: chrono::DateTime<chrono::Utc>,
}

impl From<MappedMetadataRow> for AnimeMetadata {
    fn from(r: MappedMetadataRow) -> Self {
        Self {
            id: r.id,
            mal_id: r.mal_id,
            title_romaji: r.title_romaji,
            title_english: r.title_english,
            title_native: r.title_native,
            format: r.format,
            status: r.status,
            episodes: r.episodes,
            season: r.season,
            season_year: r.season_year,
            genres: r.genres,
            average_score: r.average_score,
            description: r.description,
            cover_image_url: r.cover_image_url,
            cover_image_local_path: r.cover_image_local_path,
            banner_image_url: r.banner_image_url,
            last_synced_at: r.last_synced_at,
        }
    }
}

pub struct AniListClient {
    client: reqwest::Client,
    graphql_url: String,
    covers_dir: PathBuf,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    sync_tx: mpsc::UnboundedSender<String>,
    sync_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<String>>>>,
    in_flight_titles: Arc<Mutex<HashSet<String>>>,
}

impl AniListClient {
    pub fn new(graphql_url: String, covers_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let (sync_tx, sync_rx) = mpsc::unbounded_channel::<String>();

        let rate_limiter = Arc::new(Mutex::new(RateLimiter {
            last_request: Instant::now()
                .checked_sub(Duration::from_secs(5))
                .unwrap_or_else(Instant::now),
            retry_after_until: None,
            // Spaced out at 1000ms minimum interval (max 60 req/min, safely below AniList 90 req/min)
            min_interval: Duration::from_millis(1000),
        }));

        Self {
            client,
            graphql_url,
            covers_dir,
            rate_limiter,
            sync_tx,
            sync_rx: Arc::new(Mutex::new(Some(sync_rx))),
            in_flight_titles: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Spawns the dedicated asynchronous background metadata sync worker
    pub fn start_sync_worker(self: Arc<Self>, db: PgPool, ws: WsHub) {
        tokio::spawn(async move {
            let rx_opt = {
                let mut guard = self.sync_rx.lock().await;
                guard.take()
            };

            if let Some(mut rx) = rx_opt {
                tracing::info!("AniList async metadata sync worker initialized");
                while let Some(title) = rx.recv().await {
                    // Check if we are currently in 429 backoff
                    let wait_duration = {
                        let rl = self.rate_limiter.lock().await;
                        rl.retry_after_until.and_then(|until| {
                            let now = Instant::now();
                            if now < until {
                                Some(until - now)
                            } else {
                                None
                            }
                        })
                    };

                    if let Some(delay) = wait_duration {
                        tracing::info!(
                            "AniList background worker sleeping for {}s due to 429 rate limit backoff",
                            delay.as_secs()
                        );
                        sleep(delay).await;
                    }

                    match self.sync_anime_metadata(&db, &title).await {
                        Ok(Some(meta)) => {
                            tracing::info!(
                                "AniList metadata resolved asynchronously for '{}': {} (ID: {})",
                                title,
                                meta.title_romaji,
                                meta.id
                            );
                            ws.broadcast(
                                "anilist_metadata_synced",
                                json!({
                                    "title": title,
                                    "metadata": meta
                                }),
                            );
                        }
                        Ok(None) => {
                            tracing::debug!("AniList metadata negative cached or unavailable for '{}'", title);
                        }
                        Err(err) => {
                            tracing::warn!("AniList async sync error for '{}': {}", title, err);
                        }
                    }

                    // Release in-flight lock for this title
                    self.in_flight_titles.lock().await.remove(&title.trim().to_lowercase());
                }
            }
        });
    }

    /// Non-blocking method to enqueue an anime title for background resolution
    pub async fn enqueue_metadata_sync(&self, anime_title: &str) {
        let cleaned = anime_title.trim().to_lowercase();
        if cleaned.is_empty() {
            return;
        }

        let mut in_flight = self.in_flight_titles.lock().await;
        if in_flight.insert(cleaned) {
            let _ = self.sync_tx.send(anime_title.trim().to_string());
        }
    }

    /// High-performance batch resolver:
    /// 1. Searches local DB (title_mappings joined with anime_metadata) in one query.
    /// 2. For unmapped titles, searches local anime_metadata table directly (romaji, english, synonyms).
    /// 3. Automatically saves newly discovered local mappings to title_mappings.
    /// 4. Only if metadata does NOT exist in local DB does it enqueue to AniList API in the background.
    pub async fn resolve_titles_batch(
        &self,
        db: &PgPool,
        raw_titles: &[String],
    ) -> HashMap<String, AnimeMetadata> {
        let mut results: HashMap<String, AnimeMetadata> = HashMap::new();
        if raw_titles.is_empty() {
            return results;
        }

        // Map cleaned title -> list of original raw titles
        let mut cleaned_to_raw: HashMap<String, Vec<String>> = HashMap::new();
        for raw in raw_titles {
            let cleaned = raw.trim().to_lowercase();
            if !cleaned.is_empty() {
                cleaned_to_raw.entry(cleaned).or_default().push(raw.clone());
            }
        }

        let all_cleaned: Vec<String> = cleaned_to_raw.keys().cloned().collect();
        if all_cleaned.is_empty() {
            return results;
        }

        // 1. Batch lookup in title_mappings + anime_metadata (JOIN)
        let mapped_rows: Vec<MappedMetadataRow> = match sqlx::query_as::<_, MappedMetadataRow>(
            r#"
            SELECT
                tm.raw_title_cleaned AS mapping_key,
                m.id, m.mal_id, m.title_romaji, m.title_english, m.title_native,
                m.format, m.status, m.episodes, m.season, m.season_year,
                m.genres, m.average_score, m.description, m.cover_image_url,
                m.cover_image_local_path, m.banner_image_url, m.last_synced_at
            FROM title_mappings tm
            INNER JOIN anime_metadata m ON tm.anilist_id = m.id
            WHERE tm.raw_title_cleaned = ANY($1)
            "#
        )
        .bind(&all_cleaned)
        .fetch_all(db)
        .await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Failed to query title_mappings batch: {}", e);
                Vec::new()
            }
        };

        let mut found_keys: HashSet<String> = HashSet::new();

        for row in mapped_rows {
            let key = row.mapping_key.clone();
            found_keys.insert(key.clone());
            let meta = AnimeMetadata::from(row);
            if let Some(raw_list) = cleaned_to_raw.get(&key) {
                for r in raw_list {
                    results.insert(r.clone(), meta.clone());
                }
            }
        }

        // Also check if any remaining titles are negatively cached (already searched & not on AniList)
        let negative_mappings: HashSet<String> = match sqlx::query_as::<_, (String,)>(
            "SELECT raw_title_cleaned FROM title_mappings WHERE raw_title_cleaned = ANY($1) AND anilist_id IS NULL"
        )
        .bind(&all_cleaned)
        .fetch_all(db)
        .await {
            Ok(neg) => neg.into_iter().map(|(k,)| k).collect(),
            Err(_) => HashSet::new(),
        };

        // 2. For unmapped titles, search directly in local anime_metadata
        let unmapped_cleaned: Vec<String> = all_cleaned
            .into_iter()
            .filter(|k| !found_keys.contains(k) && !negative_mappings.contains(k))
            .collect();

        if !unmapped_cleaned.is_empty() {
            // Direct title search in local anime_metadata by romaji or english title
            let local_matches: Vec<AnimeMetadata> = match sqlx::query_as::<_, AnimeMetadata>(
                r#"
                SELECT
                    id, mal_id, title_romaji, title_english, title_native,
                    format, status, episodes, season, season_year,
                    genres, average_score, description, cover_image_url,
                    cover_image_local_path, banner_image_url, last_synced_at
                FROM anime_metadata
                WHERE LOWER(title_romaji) = ANY($1)
                   OR LOWER(title_english) = ANY($1)
                "#
            )
            .bind(&unmapped_cleaned)
            .fetch_all(db)
            .await {
                Ok(records) => records,
                Err(e) => {
                    tracing::warn!("Failed to search local anime_metadata batch: {}", e);
                    Vec::new()
                }
            };

            for meta in local_matches {
                let romaji_lower = meta.title_romaji.to_lowercase();
                let english_lower = meta.title_english.as_deref().unwrap_or("").to_lowercase();

                // Find matching cleaned keys
                let mut matched_cleaned_keys = Vec::new();
                for key in &unmapped_cleaned {
                    if *key == romaji_lower || (!english_lower.is_empty() && *key == english_lower) {
                        matched_cleaned_keys.push(key.clone());
                    }
                }

                for key in matched_cleaned_keys {
                    found_keys.insert(key.clone());
                    if let Some(raw_list) = cleaned_to_raw.get(&key) {
                        for r in raw_list {
                            results.insert(r.clone(), meta.clone());
                        }
                    }

                    // Save local mapping to title_mappings table
                    let _ = sqlx::query(
                        "INSERT INTO title_mappings (raw_title_cleaned, anilist_id, confidence) VALUES ($1, $2, 1.0) ON CONFLICT (raw_title_cleaned) DO UPDATE SET anilist_id = EXCLUDED.anilist_id"
                    )
                    .bind(&key)
                    .bind(meta.id)
                    .execute(db)
                    .await;
                }
            }
        }

        // 3. For any titles still unmapped, attempt fuzzy / ILIKE lookup in local DB
        let still_unmapped: Vec<String> = unmapped_cleaned
            .into_iter()
            .filter(|k| !found_keys.contains(k))
            .collect();

        let mut to_enqueue_api: Vec<String> = Vec::new();

        for key in still_unmapped {
            // Fuzzy search in local anime_metadata
            let fuzzy_match: Option<AnimeMetadata> = sqlx::query_as::<_, AnimeMetadata>(
                r#"
                SELECT
                    id, mal_id, title_romaji, title_english, title_native,
                    format, status, episodes, season, season_year,
                    genres, average_score, description, cover_image_url,
                    cover_image_local_path, banner_image_url, last_synced_at
                FROM anime_metadata
                WHERE LOWER(title_romaji) LIKE '%' || $1 || '%'
                   OR LOWER(title_english) LIKE '%' || $1 || '%'
                ORDER BY average_score DESC NULLS LAST
                LIMIT 1
                "#
            )
            .bind(&key)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

            if let Some(meta) = fuzzy_match {
                // Found locally!
                found_keys.insert(key.clone());
                if let Some(raw_list) = cleaned_to_raw.get(&key) {
                    for r in raw_list {
                        results.insert(r.clone(), meta.clone());
                    }
                }

                // Save mapping
                let _ = sqlx::query(
                    "INSERT INTO title_mappings (raw_title_cleaned, anilist_id, confidence) VALUES ($1, $2, 0.9) ON CONFLICT (raw_title_cleaned) DO UPDATE SET anilist_id = EXCLUDED.anilist_id"
                )
                .bind(&key)
                .bind(meta.id)
                .execute(db)
                .await;
            } else {
                // Not found anywhere in local DB -> queue for AniList API
                if let Some(raw_list) = cleaned_to_raw.get(&key) {
                    if let Some(first_raw) = raw_list.first() {
                        to_enqueue_api.push(first_raw.clone());
                    }
                }
            }
        }

        // 4. Asynchronously enqueue missing titles to AniList API in the background
        for title in to_enqueue_api {
            self.enqueue_metadata_sync(&title).await;
        }

        results
    }

    /// Read cached metadata directly from PostgreSQL without hitting AniList API
    pub async fn get_cached_metadata(
        &self,
        db: &PgPool,
        anime_title: &str,
    ) -> Result<Option<AnimeMetadata>, AppError> {
        let cleaned_title = anime_title.trim().to_lowercase();
        if cleaned_title.is_empty() {
            return Ok(None);
        }

        let mapped_id: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT anilist_id FROM title_mappings WHERE raw_title_cleaned = $1",
        )
        .bind(&cleaned_title)
        .fetch_optional(db)
        .await?;

        if let Some((Some(anilist_id),)) = mapped_id {
            let metadata = sqlx::query_as::<_, AnimeMetadata>(
                "SELECT * FROM anime_metadata WHERE id = $1",
            )
            .bind(anilist_id)
            .fetch_optional(db)
            .await?;

            return Ok(metadata);
        }

        Ok(None)
    }

    /// Check if title has already been queried (either positively or negatively cached)
    pub async fn is_title_checked(
        &self,
        db: &PgPool,
        anime_title: &str,
    ) -> Result<bool, AppError> {
        let cleaned_title = anime_title.trim().to_lowercase();
        if cleaned_title.is_empty() {
            return Ok(true);
        }

        let mapped: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT anilist_id FROM title_mappings WHERE raw_title_cleaned = $1",
        )
        .bind(&cleaned_title)
        .fetch_optional(db)
        .await?;

        Ok(mapped.is_some())
    }

    /// Performs the actual metadata fetch with token-bucket spacing and 429 Retry-After backoff
    pub async fn sync_anime_metadata(
        &self,
        db: &PgPool,
        anime_title: &str,
    ) -> Result<Option<AnimeMetadata>, AppError> {
        let cleaned_title = anime_title.trim().to_lowercase();
        if cleaned_title.is_empty() {
            return Ok(None);
        }

        // 1. Check title_mappings cache in PostgreSQL first
        if let Some(cached) = self.get_cached_metadata(db, anime_title).await? {
            return Ok(Some(cached));
        }

        // Check if negative cache mapping already exists
        let exists_mapping: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT anilist_id FROM title_mappings WHERE raw_title_cleaned = $1",
        )
        .bind(&cleaned_title)
        .fetch_optional(db)
        .await?;

        if let Some((None,)) = exists_mapping {
            // Already queried previously and not found on AniList
            return Ok(None);
        }

        // 2. Rate Limiting Check (Handle 429 backoff and min-interval spacing)
        {
            let mut rl = self.rate_limiter.lock().await;

            // If in 429 backoff, skip and do not hammer AniList
            if let Some(until) = rl.retry_after_until {
                let now = Instant::now();
                if now < until {
                    let remaining = (until - now).as_secs();
                    tracing::info!(
                        "AniList client in 429 rate-limit backoff (remaining: {}s). Skipping query for '{}'",
                        remaining,
                        anime_title
                    );
                    return Ok(None);
                } else {
                    // Backoff duration has expired
                    rl.retry_after_until = None;
                }
            }

            // Minimum interval spacing (1000ms)
            let elapsed = rl.last_request.elapsed();
            if elapsed < rl.min_interval {
                sleep(rl.min_interval - elapsed).await;
            }
            rl.last_request = Instant::now();
        }

        // 3. Query AniList GraphQL API
        let query = r#"
            query ($search: String) {
              Media(search: $search, type: ANIME, sort: SEARCH_MATCH) {
                id
                idMal
                title {
                  romaji
                  english
                  native
                }
                format
                status
                episodes
                season
                seasonYear
                genres
                averageScore
                description
                coverImage {
                  extraLarge
                  large
                }
                bannerImage
              }
            }
        "#;

        let payload = json!({
            "query": query,
            "variables": { "search": anime_title }
        });

        let response = match self
            .client
            .post(&self.graphql_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("AniList GraphQL connection error: {}", e);
                return Ok(None);
            }
        };

        // 4. Handle HTTP Status (Special 429 handling with Retry-After)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);

            let backoff_until = Instant::now() + Duration::from_secs(retry_after_secs);
            {
                let mut rl = self.rate_limiter.lock().await;
                rl.retry_after_until = Some(backoff_until);
            }

            tracing::warn!(
                "AniList API returned 429 Too Many Requests. Backing off all outgoing requests for {}s",
                retry_after_secs
            );
            return Ok(None);
        }

        if !response.status().is_success() {
            tracing::warn!("AniList API returned non-success status: {}", response.status());
            return Ok(None);
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse AniList JSON: {}", e)))?;

        let media = match body.pointer("/data/Media") {
            Some(m) if !m.is_null() => m,
            _ => {
                // Record negative cache mapping so we don't query AniList repeatedly
                let _ = sqlx::query(
                    "INSERT INTO title_mappings (raw_title_cleaned, anilist_id) VALUES ($1, NULL) ON CONFLICT DO NOTHING"
                )
                .bind(&cleaned_title)
                .execute(db)
                .await;
                return Ok(None);
            }
        };

        let id = media["id"].as_i64().unwrap_or(0) as i32;
        let mal_id = media["idMal"].as_i64().map(|v| v as i32);
        let title_romaji = media["title"]["romaji"].as_str().unwrap_or(anime_title).to_string();
        let title_english = media["title"]["english"].as_str().map(|s| s.to_string());
        let title_native = media["title"]["native"].as_str().map(|s| s.to_string());
        let format = media["format"].as_str().map(|s| s.to_string());
        let status = media["status"].as_str().map(|s| s.to_string());
        let episodes = media["episodes"].as_i64().map(|v| v as i32);
        let season = media["season"].as_str().map(|s| s.to_string());
        let season_year = media["seasonYear"].as_i64().map(|v| v as i32);
        let genres: Vec<String> = media["genres"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|g| g.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let average_score = media["averageScore"].as_i64().map(|v| v as i32);
        let description = media["description"].as_str().map(|s| s.to_string());
        let cover_url = media["coverImage"]["large"]
            .as_str()
            .or_else(|| media["coverImage"]["extraLarge"].as_str())
            .map(|s| s.to_string());
        let banner_url = media["bannerImage"].as_str().map(|s| s.to_string());

        // 5. Download Cover Image Locally
        let local_cover_path = if let Some(ref c_url) = cover_url {
            let filename = format!("{}.jpg", id);
            let full_path = self.covers_dir.join(&filename);
            if !full_path.exists() {
                if let Err(e) = self.download_image(c_url, &full_path).await {
                    tracing::warn!("Failed to download cover image for anime {}: {}", id, e);
                }
            }
            Some(format!("/api/covers/{}", filename))
        } else {
            None
        };

        // 6. Upsert into anime_metadata table
        let record = sqlx::query_as::<_, AnimeMetadata>(
            r#"
            INSERT INTO anime_metadata (
                id, mal_id, title_romaji, title_english, title_native,
                format, status, episodes, season, season_year,
                genres, average_score, description, cover_image_url,
                cover_image_local_path, banner_image_url, last_synced_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (id) DO UPDATE SET
                title_romaji = EXCLUDED.title_romaji,
                title_english = EXCLUDED.title_english,
                average_score = EXCLUDED.average_score,
                cover_image_local_path = COALESCE(EXCLUDED.cover_image_local_path, anime_metadata.cover_image_local_path),
                last_synced_at = EXCLUDED.last_synced_at
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(mal_id)
        .bind(&title_romaji)
        .bind(&title_english)
        .bind(&title_native)
        .bind(&format)
        .bind(&status)
        .bind(episodes)
        .bind(&season)
        .bind(season_year)
        .bind(&genres)
        .bind(average_score)
        .bind(&description)
        .bind(&cover_url)
        .bind(&local_cover_path)
        .bind(&banner_url)
        .bind(Utc::now())
        .fetch_one(db)
        .await?;

        // 7. Save title mapping in PostgreSQL
        let _ = sqlx::query(
            "INSERT INTO title_mappings (raw_title_cleaned, anilist_id, confidence) VALUES ($1, $2, 1.0) ON CONFLICT (raw_title_cleaned) DO UPDATE SET anilist_id = EXCLUDED.anilist_id"
        )
        .bind(&cleaned_title)
        .bind(id)
        .execute(db)
        .await;

        Ok(Some(record))
    }

    /// Fetches anime metadata directly by AniList ID, caches it in DB, and downloads cover image
    pub async fn sync_anime_by_id(
        &self,
        db: &PgPool,
        id: i32,
    ) -> Result<Option<AnimeMetadata>, AppError> {
        // Rate limiter check
        {
            let mut rl = self.rate_limiter.lock().await;

            if let Some(until) = rl.retry_after_until {
                let now = Instant::now();
                if now < until {
                    let remaining = (until - now).as_secs();
                    tracing::info!(
                        "AniList client in 429 rate-limit backoff (remaining: {}s). Skipping ID query for {}",
                        remaining,
                        id
                    );
                    return Ok(None);
                } else {
                    rl.retry_after_until = None;
                }
            }

            let elapsed = rl.last_request.elapsed();
            if elapsed < rl.min_interval {
                sleep(rl.min_interval - elapsed).await;
            }
            rl.last_request = Instant::now();
        }

        let query = r#"
            query ($id: Int) {
              Media(id: $id, type: ANIME) {
                id
                idMal
                title {
                  romaji
                  english
                  native
                }
                format
                status
                episodes
                season
                seasonYear
                genres
                averageScore
                description
                coverImage {
                  extraLarge
                  large
                }
                bannerImage
              }
            }
        "#;

        let payload = json!({
            "query": query,
            "variables": { "id": id }
        });

        let response = match self
            .client
            .post(&self.graphql_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("AniList GraphQL connection error for ID {}: {}", id, e);
                return Ok(None);
            }
        };

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_secs = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(60);

            let backoff_until = Instant::now() + Duration::from_secs(retry_after_secs);
            {
                let mut rl = self.rate_limiter.lock().await;
                rl.retry_after_until = Some(backoff_until);
            }

            tracing::warn!(
                "AniList API returned 429 Too Many Requests. Backing off for {}s",
                retry_after_secs
            );
            return Ok(None);
        }

        if !response.status().is_success() {
            tracing::warn!(
                "AniList API returned status {} for anime ID {}",
                response.status(),
                id
            );
            return Ok(None);
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse AniList JSON: {}", e)))?;

        let media = match body.pointer("/data/Media") {
            Some(m) if !m.is_null() => m,
            _ => return Ok(None),
        };

        let mal_id = media["idMal"].as_i64().map(|v| v as i32);
        let title_romaji = media["title"]["romaji"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let title_english = media["title"]["english"].as_str().map(|s| s.to_string());
        let title_native = media["title"]["native"].as_str().map(|s| s.to_string());
        let format = media["format"].as_str().map(|s| s.to_string());
        let status = media["status"].as_str().map(|s| s.to_string());
        let episodes = media["episodes"].as_i64().map(|v| v as i32);
        let season = media["season"].as_str().map(|s| s.to_string());
        let season_year = media["seasonYear"].as_i64().map(|v| v as i32);
        let genres: Vec<String> = media["genres"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|g| g.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let average_score = media["averageScore"].as_i64().map(|v| v as i32);
        let description = media["description"].as_str().map(|s| s.to_string());
        let cover_url = media["coverImage"]["large"]
            .as_str()
            .or_else(|| media["coverImage"]["extraLarge"].as_str())
            .map(|s| s.to_string());
        let banner_url = media["bannerImage"].as_str().map(|s| s.to_string());

        let filename = format!("{}.jpg", id);
        let full_path = self.covers_dir.join(&filename);
        let local_cover_path = if let Some(ref c_url) = cover_url {
            if !full_path.exists() {
                if let Err(e) = self.download_image(c_url, &full_path).await {
                    tracing::warn!("Failed to download cover image for anime {}: {}", id, e);
                }
            }
            Some(format!("/api/covers/{}", filename))
        } else {
            None
        };

        let record = sqlx::query_as::<_, AnimeMetadata>(
            r#"
            INSERT INTO anime_metadata (
                id, mal_id, title_romaji, title_english, title_native,
                format, status, episodes, season, season_year,
                genres, average_score, description, cover_image_url,
                cover_image_local_path, banner_image_url, last_synced_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (id) DO UPDATE SET
                title_romaji = EXCLUDED.title_romaji,
                title_english = EXCLUDED.title_english,
                average_score = EXCLUDED.average_score,
                cover_image_url = COALESCE(EXCLUDED.cover_image_url, anime_metadata.cover_image_url),
                cover_image_local_path = COALESCE(EXCLUDED.cover_image_local_path, anime_metadata.cover_image_local_path),
                last_synced_at = EXCLUDED.last_synced_at
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(mal_id)
        .bind(&title_romaji)
        .bind(&title_english)
        .bind(&title_native)
        .bind(&format)
        .bind(&status)
        .bind(episodes)
        .bind(&season)
        .bind(season_year)
        .bind(&genres)
        .bind(average_score)
        .bind(&description)
        .bind(&cover_url)
        .bind(&local_cover_path)
        .bind(&banner_url)
        .bind(Utc::now())
        .fetch_one(db)
        .await?;

        Ok(Some(record))
    }

    /// Redownloads a cover image from its source (DB cached URL or AniList API)
    /// when the local cover file does not exist.
    pub async fn redownload_cover(&self, db: &PgPool, filename: &str) -> Result<Vec<u8>, AppError> {
        let clean_filename = filename.replace("..", "").replace(['/', '\\'], "");
        let file_path = self.covers_dir.join(&clean_filename);

        if file_path.exists() {
            if let Ok(bytes) = tokio::fs::read(&file_path).await {
                return Ok(bytes);
            }
        }

        let id_opt = Path::new(&clean_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse::<i32>().ok())
            .or_else(|| {
                clean_filename
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<i32>()
                    .ok()
            });

        // 1. Check if cover_image_url is in anime_metadata by ID
        let mut cover_url: Option<String> = None;
        if let Some(id) = id_opt {
            let row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT cover_image_url FROM anime_metadata WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(db)
            .await
            .unwrap_or(None);

            if let Some((url,)) = row {
                cover_url = url;
            }
        }

        // 2. If not found by ID, check by cover_image_local_path
        if cover_url.is_none() {
            let row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT cover_image_url FROM anime_metadata WHERE cover_image_local_path = $1 OR cover_image_local_path LIKE '%' || $1 LIMIT 1",
            )
            .bind(&clean_filename)
            .fetch_optional(db)
            .await
            .unwrap_or(None);

            if let Some((url,)) = row {
                cover_url = url;
            }
        }

        // 3. If cover_url was found, download it
        if let Some(ref url) = cover_url {
            if !url.trim().is_empty() {
                match self.download_image(url, &file_path).await {
                    Ok(bytes) => {
                        if let Some(id) = id_opt {
                            let _ = sqlx::query(
                                "UPDATE anime_metadata SET cover_image_local_path = $1 WHERE id = $2 AND (cover_image_local_path IS NULL OR cover_image_local_path = '')",
                            )
                            .bind(format!("/api/covers/{}", clean_filename))
                            .bind(id)
                            .execute(db)
                            .await;
                        }
                        return Ok(bytes);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to download cover from known URL '{}': {}",
                            url,
                            e
                        );
                    }
                }
            }
        }

        // 4. If cover_url was not in DB (or download failed), and we have an AniList ID:
        // Query AniList API directly by ID
        if let Some(id) = id_opt {
            if let Ok(Some(meta)) = self.sync_anime_by_id(db, id).await {
                if file_path.exists() {
                    if let Ok(bytes) = tokio::fs::read(&file_path).await {
                        return Ok(bytes);
                    }
                }
                if let Some(ref url) = meta.cover_image_url {
                    if let Ok(bytes) = self.download_image(url, &file_path).await {
                        return Ok(bytes);
                    }
                }
            }
        }

        Err(AppError::NotFound("Cover image not found".into()))
    }

    pub async fn download_image(&self, url: &str, path: &Path) -> Result<Vec<u8>, AppError> {
        let res = self.client.get(url).send().await.map_err(|e| {
            AppError::Internal(format!("Failed to download image from {}: {}", url, e))
        })?;

        if !res.status().is_success() {
            return Err(AppError::NotFound(format!(
                "Failed to download image from {}, status: {}",
                url,
                res.status()
            )));
        }

        let bytes = res.bytes().await.map_err(|e| {
            AppError::Internal(format!("Failed to read image bytes: {}", e))
        })?.to_vec();

        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let tmp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        if let Ok(()) = tokio::fs::write(&tmp_path, &bytes).await {
            if let Err(_e) = tokio::fs::rename(&tmp_path, path).await {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                let _ = tokio::fs::write(path, &bytes).await;
            }
        } else {
            let _ = tokio::fs::write(path, &bytes).await;
        }

        Ok(bytes)
    }
}
