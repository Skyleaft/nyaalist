use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LibraryFile {
    pub id: Uuid,
    pub download_job_id: Option<Uuid>,
    pub anilist_id: Option<i32>,
    pub original_filename: String,
    pub current_path: String,
    pub target_library_path: Option<String>,
    pub episode: Option<String>,
    pub season: Option<i32>,
    pub resolution: Option<String>,
    pub file_size_bytes: i64,
    pub is_moved: bool,
    pub is_encoded: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryFileItem {
    #[serde(flatten)]
    pub file: LibraryFile,
    pub anime_title: Option<String>,
    pub anime_cover_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveFileRequest {
    pub file_id: Option<Uuid>,
    pub source_path: String,
    pub target_path: String,
    pub new_filename: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoOrganizeRequest {
    pub file_id: Option<Uuid>,
    pub download_job_id: Option<Uuid>,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncodeFileRequest {
    pub preset_id: Option<Uuid>,
    pub target_file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslateFileRequest {
    pub source_stream_index: Option<i32>,
    pub source_language: Option<String>,
    pub target_language: String,
    pub target_language_code: Option<String>,
    pub gemini_model: Option<String>,
    pub embed_mode: Option<String>,
    pub target_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDirectoryResult {
    pub scanned_count: usize,
    pub new_files_count: usize,
    pub files: Vec<LibraryFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibraryFilesQuery {
    pub is_moved: Option<bool>,
    pub search: Option<String>,
}
