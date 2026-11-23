use std::path::PathBuf;
use crate::core::error::AppError;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub app_env: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub database_min_connections: u32,
    pub database_timeout_seconds: u64,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub cookie_secure: bool,
    pub cookie_name: String,
    pub downloads_dir: PathBuf,
    pub library_dir: PathBuf,
    pub covers_dir: PathBuf,
    pub temp_encode_dir: PathBuf,
    pub nyaa_base_url: String,
    pub nyaa_default_category: String,
    pub nyaa_auto_sync_interval_mins: u64,
    pub nyaa_user_agent: String,
    pub nyaa_request_timeout_seconds: u64,
    pub anilist_graphql_url: String,
    pub anilist_rate_limit: u32,
    pub anilist_request_timeout_seconds: u64,
    pub torrent_listen_port: u16,
    pub torrent_enable_dht: bool,
    pub torrent_max_active_downloads: usize,
    pub torrent_max_peers_per_torrent: usize,
    pub torrent_global_download_limit_kbps: u64,
    pub torrent_global_upload_limit_kbps: u64,
    pub ffmpeg_binary_path: String,
    pub ffprobe_binary_path: String,
    pub encoder_max_concurrent_jobs: usize,
    pub encoder_default_preset: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        // Loads .env file if present, silently ignores if missing (e.g. in Docker)
        let _ = dotenvy::dotenv();

        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| AppError::Config(format!("Invalid PORT: {}", e)))?,
            app_env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://nyaa_user:nyaa_secure_password@localhost:5432/nyaalist".to_string()),
            database_max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20),
            database_min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            database_timeout_seconds: std::env::var("DATABASE_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "nyaalist_super_secret_jwt_random_key_change_me_in_production_min_32_bytes".to_string()),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "72".to_string())
                .parse()
                .unwrap_or(72),
            cookie_secure: std::env::var("COOKIE_SECURE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            cookie_name: std::env::var("COOKIE_NAME")
                .unwrap_or_else(|_| "nyaalist_session".to_string()),
            downloads_dir: PathBuf::from(std::env::var("DOWNLOADS_DIR")
                .unwrap_or_else(|_| "./data/downloads".to_string())),
            library_dir: PathBuf::from(std::env::var("LIBRARY_DIR")
                .unwrap_or_else(|_| "./data/library".to_string())),
            covers_dir: PathBuf::from(std::env::var("COVERS_DIR")
                .unwrap_or_else(|_| "./data/covers".to_string())),
            temp_encode_dir: PathBuf::from(std::env::var("TEMP_ENCODE_DIR")
                .unwrap_or_else(|_| "./data/temp_encode".to_string())),
            nyaa_base_url: std::env::var("NYAA_BASE_URL")
                .unwrap_or_else(|_| "https://nyaa.si".to_string()),
            nyaa_default_category: std::env::var("NYAA_DEFAULT_CATEGORY")
                .unwrap_or_else(|_| "1_2".to_string()),
            nyaa_auto_sync_interval_mins: std::env::var("NYAA_AUTO_SYNC_INTERVAL_MINS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            nyaa_user_agent: std::env::var("NYAA_USER_AGENT")
                .unwrap_or_else(|_| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Nyaalist/1.0".to_string()),
            nyaa_request_timeout_seconds: std::env::var("NYAA_REQUEST_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .unwrap_or(15),
            anilist_graphql_url: std::env::var("ANILIST_GRAPHQL_URL")
                .unwrap_or_else(|_| "https://graphql.anilist.co".to_string()),
            anilist_rate_limit: std::env::var("ANILIST_RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "90".to_string())
                .parse()
                .unwrap_or(90),
            anilist_request_timeout_seconds: std::env::var("ANILIST_REQUEST_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            torrent_listen_port: std::env::var("TORRENT_LISTEN_PORT")
                .unwrap_or_else(|_| "4240".to_string())
                .parse()
                .unwrap_or(4240),
            torrent_enable_dht: std::env::var("TORRENT_ENABLE_DHT")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            torrent_max_active_downloads: std::env::var("TORRENT_MAX_ACTIVE_DOWNLOADS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            torrent_max_peers_per_torrent: std::env::var("TORRENT_MAX_PEERS_PER_TORRENT")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .unwrap_or(50),
            torrent_global_download_limit_kbps: std::env::var("TORRENT_GLOBAL_DOWNLOAD_LIMIT_KBPS")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            torrent_global_upload_limit_kbps: std::env::var("TORRENT_GLOBAL_UPLOAD_LIMIT_KBPS")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            ffmpeg_binary_path: std::env::var("FFMPEG_BINARY_PATH")
                .unwrap_or_else(|_| "ffmpeg".to_string()),
            ffprobe_binary_path: std::env::var("FFPROBE_BINARY_PATH")
                .unwrap_or_else(|_| "ffprobe".to_string()),
            encoder_max_concurrent_jobs: std::env::var("ENCODER_MAX_CONCURRENT_JOBS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
            encoder_default_preset: std::env::var("ENCODER_DEFAULT_PRESET")
                .unwrap_or_else(|_| "720p x265 Balanced".to_string()),
        })
    }
}
