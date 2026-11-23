use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct AddTorrentRequest {
    pub magnet_uri: String,
    pub nyaa_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteTorrentQuery {
    pub delete_files: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentStatsSnapshot {
    pub id: Uuid,
    pub info_hash: String,
    pub name: String,
    pub status: String, // QUEUED, DOWNLOADING, PAUSED, COMPLETED, ERROR
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub uploaded_bytes: i64,
    pub download_speed: i64, // bytes/sec
    pub upload_speed: i64,   // bytes/sec
    pub progress_pct: f32,
    pub eta_seconds: Option<i64>,
    pub seeders: i32,
    pub peers: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DownloadJob {
    pub id: Uuid,
    pub nyaa_torrent_id: Option<i64>,
    pub info_hash: String,
    pub name: String,
    pub magnet_uri: String,
    pub save_path: String,
    pub status: String,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub uploaded_bytes: i64,
    pub download_speed: i64,
    pub upload_speed: i64,
    pub progress_pct: f32,
    pub eta_seconds: Option<i64>,
    pub seeders: i32,
    pub peers: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
