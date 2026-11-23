use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "anime_metadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    pub mal_id: Option<i32>,
    pub title_romaji: String,
    pub title_english: Option<String>,
    pub title_native: Option<String>,
    pub synonyms: Option<Vec<String>>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub episodes: Option<i32>,
    pub season: Option<String>,
    pub season_year: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub average_score: Option<i32>,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub cover_image_local_path: Option<String>,
    pub banner_image_url: Option<String>,
    pub last_synced_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::nyaa_torrents::Entity")]
    NyaaTorrents,
    #[sea_orm(has_many = "super::library_files::Entity")]
    LibraryFiles,
}

impl Related<super::nyaa_torrents::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NyaaTorrents.def()
    }
}

impl Related<super::library_files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LibraryFiles.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
