use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "title_mappings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub raw_title_cleaned: String,
    pub anilist_id: Option<i32>,
    pub confidence: f32,
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
}

impl Related<super::anime_metadata::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AnimeMetadata.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
