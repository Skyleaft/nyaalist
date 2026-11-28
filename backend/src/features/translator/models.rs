use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub stream_index: i32,
    pub sub_index: i32,
    pub codec_name: String,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_text_based: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProbeSubtitlesRequest {
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSubtitlesResponse {
    pub file_path: String,
    pub tracks: Vec<SubtitleTrack>,
    pub recommended_stream_index: Option<i32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TranslationJob {
    pub id: Uuid,
    pub library_file_id: Option<Uuid>,
    pub source_file_path: String,
    pub target_file_path: String,
    pub extracted_sub_path: Option<String>,
    pub translated_sub_path: Option<String>,
    pub source_stream_index: i32,
    pub source_sub_codec: Option<String>,
    pub source_language: Option<String>,
    pub target_language: String,
    pub target_language_code: String,
    pub gemini_model: String,
    pub embed_mode: String,
    pub status: String, // QUEUED, EXTRACTING, TRANSLATING, EMBEDDING, COMPLETED, FAILED, CANCELLED
    pub progress_pct: f32,
    pub total_cues: i32,
    pub translated_cues: i32,
    pub current_batch: i32,
    pub total_batches: i32,
    pub stage_message: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationJobItem {
    #[serde(flatten)]
    pub job: TranslationJob,
    pub source_filename: String,
    pub target_filename: String,
    pub has_translated_sub: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationProgressTelemetry {
    pub job_id: Uuid,
    pub file_name: String,
    pub target_language: String,
    pub gemini_model: String,
    pub status: String,
    pub progress_pct: f32,
    pub total_cues: i32,
    pub translated_cues: i32,
    pub current_batch: i32,
    pub total_batches: i32,
    pub stage_message: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatorStatusResponse {
    pub gemini_configured: bool,
    pub default_model: String,
    pub ffmpeg_available: bool,
    pub ffprobe_available: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestGeminiKeyRequest {
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueTranslationRequest {
    pub library_file_id: Option<Uuid>,
    pub source_file_path: String,
    pub target_file_path: Option<String>,
    pub source_stream_index: Option<i32>,
    pub source_language: Option<String>,
    pub target_language: String,
    pub target_language_code: Option<String>,
    pub gemini_model: Option<String>,
    pub embed_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranslationJobsFilterQuery {
    pub status: Option<String>,
}

pub fn infer_language_code(target_language: &str) -> String {
    let norm = target_language.trim().to_lowercase();
    match norm.as_str() {
        "indonesian" | "bahasa indonesia" | "indonesia" | "id" | "ind" => "ind".into(),
        "english" | "en" | "eng" => "eng".into(),
        "japanese" | "nihongo" | "ja" | "jp" | "jpn" => "jpn".into(),
        "malay" | "bahasa melayu" | "ms" | "msa" | "may" => "msa".into(),
        "spanish" | "espanol" | "español" | "es" | "spa" => "spa".into(),
        "french" | "francais" | "français" | "fr" | "fre" | "fra" => "fre".into(),
        "german" | "deutsch" | "de" | "ger" | "deu" => "ger".into(),
        "korean" | "ko" | "kor" => "kor".into(),
        "chinese" | "chinese (simplified)" | "chinese (traditional)" | "mandarin" | "zh" | "chi" | "zho" => "chi".into(),
        "arabic" | "ar" | "ara" => "ara".into(),
        "thai" | "th" | "tha" => "tha".into(),
        "vietnamese" | "vi" | "vie" => "vie".into(),
        "portuguese" | "pt" | "por" => "por".into(),
        "russian" | "ru" | "rus" => "rus".into(),
        "italian" | "it" | "ita" => "ita".into(),
        "filipino" | "tagalog" | "tl" | "fil" | "tgl" => "fil".into(),
        _ => {
            let alpha: String = norm.chars().filter(|c| c.is_ascii_alphabetic()).take(3).collect();
            if alpha.len() >= 2 {
                alpha
            } else {
                "und".into()
            }
        }
    }
}
