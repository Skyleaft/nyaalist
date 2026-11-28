use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfigDto {
    pub downloads_dir: String,
    pub library_dir: String,
    pub covers_dir: String,
    pub temp_encode_dir: String,
    pub nyaa_base_url: String,
    pub nyaa_default_category: String,
    pub anilist_graphql_url: String,
    pub torrent_listen_port: u16,
    pub torrent_enable_dht: bool,
    pub torrent_global_download_limit_kbps: u64,
    pub torrent_global_upload_limit_kbps: u64,
    pub ffmpeg_binary_path: String,
    pub ffprobe_binary_path: String,
    pub encoder_default_preset: String,
    pub gemini_api_key_configured: bool,
    pub gemini_api_key_masked: Option<String>,
    pub gemini_model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfigRequest {
    pub downloads_dir: Option<String>,
    pub library_dir: Option<String>,
    pub nyaa_default_category: Option<String>,
    pub torrent_global_download_limit_kbps: Option<u64>,
    pub torrent_global_upload_limit_kbps: Option<u64>,
    pub encoder_default_preset: Option<String>,
    pub gemini_api_key: Option<String>,
    pub clear_gemini_api_key: Option<bool>,
    pub gemini_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidatePathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationResponse {
    pub path: String,
    pub exists: bool,
    pub is_dir: bool,
    pub writable: bool,
}
