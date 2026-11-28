use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS translation_jobs (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                library_file_id UUID REFERENCES library_files(id) ON DELETE SET NULL,
                source_file_path TEXT NOT NULL,
                target_file_path TEXT NOT NULL,
                extracted_sub_path TEXT,
                translated_sub_path TEXT,
                source_stream_index INT NOT NULL DEFAULT 0,
                source_sub_codec VARCHAR(32),
                source_language VARCHAR(32),
                target_language VARCHAR(64) NOT NULL,
                target_language_code VARCHAR(16) NOT NULL DEFAULT 'ind',
                gemini_model VARCHAR(64) NOT NULL DEFAULT 'gemini-2.5-flash',
                embed_mode VARCHAR(32) NOT NULL DEFAULT 'mux_keep_original',
                status VARCHAR(32) NOT NULL DEFAULT 'QUEUED',
                progress_pct REAL NOT NULL DEFAULT 0.0,
                total_cues INT NOT NULL DEFAULT 0,
                translated_cues INT NOT NULL DEFAULT 0,
                current_batch INT NOT NULL DEFAULT 0,
                total_batches INT NOT NULL DEFAULT 0,
                stage_message TEXT,
                error_message TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                started_at TIMESTAMPTZ,
                completed_at TIMESTAMPTZ
            );

            CREATE INDEX IF NOT EXISTS idx_translation_jobs_status ON translation_jobs (status);
            CREATE INDEX IF NOT EXISTS idx_translation_jobs_created_at ON translation_jobs (created_at DESC);
        "#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DROP TABLE IF EXISTS translation_jobs CASCADE;
        "#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
