use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EncodingPreset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub video_codec: String,
    pub preset: String,
    pub crf: i32,
    pub target_resolution: Option<String>,
    pub audio_codec: String,
    pub audio_bitrate: Option<String>,
    pub container: String,
    pub custom_ffmpeg_args: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EncodingJob {
    pub id: Uuid,
    pub source_file_path: String,
    pub target_file_path: String,
    pub preset_id: Uuid,
    pub status: String, // QUEUED, ENCODING, COMPLETED, FAILED, CANCELLED
    pub progress_pct: f32,
    pub current_frame: i64,
    pub total_frames: Option<i64>,
    pub fps: f32,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    pub speed: f32,
    pub source_size_bytes: i64,
    pub encoded_size_bytes: i64,
    pub eta_seconds: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingJobItem {
    #[serde(flatten)]
    pub job: EncodingJob,
    pub preset_name: String,
    pub source_filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeProgressTelemetry {
    pub job_id: Uuid,
    pub file_name: String,
    pub preset_name: String,
    pub status: String,
    pub progress_pct: f32,
    pub current_frame: i64,
    pub fps: f32,
    pub speed: f32,
    pub current_time_sec: f32,
    pub total_duration_sec: f32,
    pub eta_seconds: Option<i64>,
    pub source_size_bytes: i64,
    pub current_size_bytes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePresetRequest {
    pub name: String,
    pub description: Option<String>,
    pub video_codec: Option<String>,
    pub preset: Option<String>,
    pub crf: Option<i32>,
    pub target_resolution: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_bitrate: Option<String>,
    pub container: Option<String>,
    pub custom_ffmpeg_args: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueJobRequest {
    pub source_file_path: String,
    pub preset_id: Option<Uuid>,
    pub target_file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobsFilterQuery {
    pub status: Option<String>,
}
