use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "nyaa_torrents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub title: String,
    pub uploader: Option<String>,
    pub parsed_anime_title: Option<String>,
    pub parsed_episode: Option<String>,
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_audio: Option<String>,
    pub parsed_hash: Option<String>,
    pub magnet_uri: String,
    pub torrent_file_url: Option<String>,
    pub category: String,
    pub file_size_bytes: i64,
    pub seeders: i32,
    pub leechers: i32,
    pub completed_count: i32,
    pub nyaa_created_at: DateTime<Utc>,
    pub anilist_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::anime_metadata::Entity",
        from = "Column::AnilistId",
        to = "super::anime_metadata::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    AnimeMetadata,
    #[sea_orm(has_many = "super::download_jobs::Entity")]
    DownloadJobs,
}

impl Related<super::anime_metadata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AnimeMetadata.def()
    }
}

impl Related<super::download_jobs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DownloadJobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
