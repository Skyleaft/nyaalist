use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "encoding_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub source_file_path: String,
    pub target_file_path: String,
    pub preset_id: Uuid,
    pub status: String,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::encoding_presets::Entity",
        from = "Column::PresetId",
        to = "super::encoding_presets::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    EncodingPreset,
}

impl Related<super::encoding_presets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EncodingPreset.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
