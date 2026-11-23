use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "download_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub nyaa_torrent_id: Option<i64>,
    #[sea_orm(unique)]
    pub info_hash: String,
    pub name: String,
    pub magnet_uri: String,
    pub save_path: String,
    pub status: String,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub uploaded_bytes: i64,
    pub download_speed: i64,
    pub upload_speed: i64,
    pub progress_pct: f32,
    pub eta_seconds: Option<i64>,
    pub seeders: i32,
    pub peers: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::nyaa_torrents::Entity",
        from = "Column::NyaaTorrentId",
        to = "super::nyaa_torrents::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    NyaaTorrent,
    #[sea_orm(has_many = "super::library_files::Entity")]
    LibraryFiles,
}

impl Related<super::nyaa_torrents::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NyaaTorrent.def()
    }
}

impl Related<super::library_files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LibraryFiles.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
