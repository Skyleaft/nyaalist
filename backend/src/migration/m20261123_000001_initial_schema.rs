use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20261123_001_init.sql");
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DROP TABLE IF EXISTS encoding_jobs CASCADE;
            DROP TABLE IF EXISTS encoding_presets CASCADE;
            DROP TABLE IF EXISTS library_files CASCADE;
            DROP TABLE IF EXISTS download_jobs CASCADE;
            DROP TABLE IF EXISTS anime_torrent_matches CASCADE;
            DROP TABLE IF EXISTS nyaa_torrents CASCADE;
            DROP TABLE IF EXISTS anime_metadata CASCADE;
            DROP TABLE IF EXISTS app_config CASCADE;
            DROP TABLE IF EXISTS users CASCADE;
        "#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
