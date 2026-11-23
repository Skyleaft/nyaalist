use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRelease {
    pub uploader: Option<String>,
    pub anime_title: String,
    pub episode: Option<String>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub audio: Option<String>,
    pub source: Option<String>,
    pub crc32: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AnimeMetadata {
    pub id: i32,
    pub mal_id: Option<i32>,
    pub title_romaji: String,
    pub title_english: Option<String>,
    pub title_native: Option<String>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub episodes: Option<i32>,
    pub season: Option<String>,
    pub season_year: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub average_score: Option<i32>,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub cover_image_local_path: Option<String>,
    pub banner_image_url: Option<String>,
    pub last_synced_at: DateTime<Utc>,
}

impl From<crate::entities::anime_metadata::Model> for AnimeMetadata {
    fn from(m: crate::entities::anime_metadata::Model) -> Self {
        Self {
            id: m.id,
            mal_id: m.mal_id,
            title_romaji: m.title_romaji,
            title_english: m.title_english,
            title_native: m.title_native,
            format: m.format,
            status: m.status,
            episodes: m.episodes,
            season: m.season,
            season_year: m.season_year,
            genres: m.genres,
            average_score: m.average_score,
            description: m.description,
            cover_image_url: m.cover_image_url,
            cover_image_local_path: m.cover_image_local_path,
            banner_image_url: m.banner_image_url,
            last_synced_at: m.last_synced_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NyaaRelease {
    pub id: i64,
    pub title: String,
    pub uploader: Option<String>,
    pub magnet_uri: String,
    pub torrent_file_url: Option<String>,
    pub category: String,
    pub file_size_bytes: i64,
    pub file_size_formatted: String,
    pub seeders: i32,
    pub leechers: i32,
    pub completed_count: i32,
    pub nyaa_created_at: DateTime<Utc>,
    pub parsed: ParsedRelease,
    pub anilist_metadata: Option<AnimeMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct ExploreQuery {
    pub category: Option<String>,
    pub page: Option<u32>,
    pub query: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub title: String,
    pub nyaa_id: Option<i64>,
}
