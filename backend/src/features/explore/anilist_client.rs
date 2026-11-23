use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

use crate::core::error::AppError;
use crate::features::explore::models::AnimeMetadata;

pub struct AniListClient {
    client: reqwest::Client,
    graphql_url: String,
    last_request_time: Arc<Mutex<Instant>>,
    covers_dir: std::path::PathBuf,
}

impl AniListClient {
    pub fn new(graphql_url: String, covers_dir: std::path::PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            graphql_url,
            last_request_time: Arc::new(Mutex::new(Instant::now().checked_sub(Duration::from_secs(5)).unwrap_or_else(Instant::now))),
            covers_dir,
        }
    }

    pub async fn sync_anime_metadata(
        &self,
        db: &PgPool,
        anime_title: &str,
    ) -> Result<Option<AnimeMetadata>, AppError> {
        let cleaned_title = anime_title.trim().to_lowercase();
        if cleaned_title.is_empty() {
            return Ok(None);
        }

        // 1. Check title_mappings cache in PostgreSQL
        let mapped_id: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT anilist_id FROM title_mappings WHERE raw_title_cleaned = $1"
        )
        .bind(&cleaned_title)
        .fetch_optional(db)
        .await?;

        if let Some((Some(anilist_id),)) = mapped_id {
            let metadata = sqlx::query_as::<_, AnimeMetadata>(
                "SELECT * FROM anime_metadata WHERE id = $1"
            )
            .bind(anilist_id)
            .fetch_optional(db)
            .await?;

            if let Some(meta) = metadata {
                return Ok(Some(meta));
            }
        }

        // 2. Rate Limiting Check (Token-bucket spacing to stay under 90 req/min)
        let mut last_req = self.last_request_time.lock().await;
        let elapsed = last_req.elapsed();
        if elapsed < Duration::from_millis(700) {
            sleep(Duration::from_millis(700) - elapsed).await;
        }
        *last_req = Instant::now();
        drop(last_req);

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

        let response = self
            .client
            .post(&self.graphql_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("AniList GraphQL query error: {}", e)))?;

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

        // 4. Download Cover Image Locally
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

        // 5. Upsert into anime_metadata table
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

        // 6. Save title mapping
        let _ = sqlx::query(
            "INSERT INTO title_mappings (raw_title_cleaned, anilist_id, confidence) VALUES ($1, $2, 1.0) ON CONFLICT (raw_title_cleaned) DO UPDATE SET anilist_id = EXCLUDED.anilist_id"
        )
        .bind(&cleaned_title)
        .bind(id)
        .execute(db)
        .await;

        Ok(Some(record))
    }

    async fn download_image(&self, url: &str, path: &Path) -> Result<(), AppError> {
        let res = self.client.get(url).send().await.map_err(|e| {
            AppError::Internal(format!("Failed to download image from {}: {}", url, e))
        })?;

        if !res.status().is_success() {
            return Err(AppError::Internal(format!(
                "Failed to download image, status: {}",
                res.status()
            )));
        }

        let bytes = res.bytes().await.map_err(|e| {
            AppError::Internal(format!("Failed to read image bytes: {}", e))
        })?;

        tokio::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).await?;
        tokio::fs::write(path, bytes).await?;

        Ok(())
    }
}
