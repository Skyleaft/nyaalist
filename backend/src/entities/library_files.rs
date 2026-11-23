use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "library_files")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::download_jobs::Entity",
        from = "Column::DownloadJobId",
        to = "super::download_jobs::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    DownloadJob,
    #[sea_orm(
        belongs_to = "super::anime_metadata::Entity",
        from = "Column::AnilistId",
        to = "super::anime_metadata::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    AnimeMetadata,
}

impl Related<super::download_jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DownloadJob.def()
    }
}

impl Related<super::anime_metadata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AnimeMetadata.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
