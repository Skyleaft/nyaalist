use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "translation_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
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
    pub status: String,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
