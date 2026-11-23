use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "encoding_presets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::encoding_jobs::Entity")]
    EncodingJobs,
}

impl Related<super::encoding_jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EncodingJobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
