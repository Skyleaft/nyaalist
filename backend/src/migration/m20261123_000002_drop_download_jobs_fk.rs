use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            ALTER TABLE download_jobs DROP CONSTRAINT IF EXISTS download_jobs_nyaa_torrent_id_fkey;
        "#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_constraint WHERE conname = 'download_jobs_nyaa_torrent_id_fkey'
                ) THEN
                    ALTER TABLE download_jobs
                    ADD CONSTRAINT download_jobs_nyaa_torrent_id_fkey
                    FOREIGN KEY (nyaa_torrent_id) REFERENCES nyaa_torrents(id) ON DELETE SET NULL;
                END IF;
            END $$;
        "#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
