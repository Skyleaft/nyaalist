use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use sea_orm::DatabaseConnection;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::ws::WsHub;
use crate::features::config::routes::resolve_gemini_settings;
use crate::features::translator::ffmpeg_sub::{
    embed_subtitle, extract_subtitle, pick_recommended_track, probe_subtitle_tracks,
};
use crate::features::translator::gemini_client::GeminiClient;
use crate::features::translator::models::{TranslationJob, TranslationProgressTelemetry};
use crate::features::translator::subtitle::{ParsedSubtitle, SubtitleFormat};

const TRANSLATION_BATCH_SIZE: usize = 45;

pub struct TranslationQueueWorker {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub work_dir: PathBuf,
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub sea_db: DatabaseConnection,
    pub ws: WsHub,
    pub gemini: GeminiClient,
}

impl TranslationQueueWorker {
    pub fn new(
        ffmpeg_path: String,
        ffprobe_path: String,
        work_dir: PathBuf,
        config: Arc<AppConfig>,
        db: PgPool,
        sea_db: DatabaseConnection,
        ws: WsHub,
    ) -> Arc<Self> {
        Arc::new(Self {
            ffmpeg_path,
            ffprobe_path,
            work_dir,
            config,
            db,
            sea_db,
            ws,
            gemini: GeminiClient::new(),
        })
    }

    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            info!("Starting AI Subtitle Translator Queue Worker...");

            // Recover jobs stuck in active stages from a previous server restart
            let cleanup_res = sqlx::query(
                r#"
                UPDATE translation_jobs
                SET
                    status = 'FAILED',
                    error_message = 'Translation interrupted by backend restart',
                    completed_at = NOW()
                WHERE status IN ('EXTRACTING', 'TRANSLATING', 'EMBEDDING')
                "#,
            )
            .execute(&self.db)
            .await;

            match cleanup_res {
                Ok(res) if res.rows_affected() > 0 => {
                    warn!(
                        "Recovered {} interrupted translation job(s) as FAILED",
                        res.rows_affected()
                    );
                }
                Err(e) => {
                    error!("Failed recovering stuck translation jobs: {e}");
                }
                _ => {}
            }

            loop {
                let next_job = sqlx::query_as::<_, TranslationJob>(
                    r#"
                    SELECT
                        id, library_file_id, source_file_path, target_file_path,
                        extracted_sub_path, translated_sub_path, source_stream_index,
                        source_sub_codec, source_language, target_language,
                        target_language_code, gemini_model, embed_mode, status,
                        progress_pct, total_cues, translated_cues, current_batch,
                        total_batches, stage_message, error_message,
                        created_at, started_at, completed_at
                    FROM translation_jobs
                    WHERE status = 'QUEUED'
                    ORDER BY created_at ASC
                    LIMIT 1
                    "#,
                )
                .fetch_optional(&self.db)
                .await;

                match next_job {
                    Ok(Some(job)) => {
                        info!(
                            "Picked up AI translation job {} for '{}' -> {}",
                            job.id, job.source_file_path, job.target_language
                        );
                        if let Err(e) = self.process_job(&job).await {
                            let err_msg = e.to_string();
                            error!("Translation job {} failed: {}", job.id, err_msg);

                            let _ = sqlx::query(
                                r#"
                                UPDATE translation_jobs
                                SET status = 'FAILED', error_message = $1, stage_message = 'Failed', completed_at = NOW()
                                WHERE id = $2 AND status != 'CANCELLED'
                                "#,
                            )
                            .bind(&err_msg)
                            .bind(job.id)
                            .execute(&self.db)
                            .await;

                            self.broadcast_telemetry(
                                &job,
                                "FAILED",
                                job.progress_pct,
                                job.total_cues,
                                job.translated_cues,
                                job.current_batch,
                                job.total_batches,
                                Some("Translation failed"),
                                Some(&err_msg),
                            );
                        }
                    }
                    Ok(None) => {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        error!("Database error querying translation_jobs: {e}");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    async fn is_cancelled(&self, job_id: Uuid) -> bool {
        sqlx::query_scalar::<_, String>("SELECT status FROM translation_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_optional(&self.db)
            .await
            .ok()
            .flatten()
            .is_some_and(|s| s == "CANCELLED")
    }

    async fn update_stage(
        &self,
        job: &TranslationJob,
        status: &str,
        progress_pct: f32,
        total_cues: i32,
        translated_cues: i32,
        current_batch: i32,
        total_batches: i32,
        stage_message: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE translation_jobs
            SET
                status = $1,
                progress_pct = $2,
                total_cues = $3,
                translated_cues = $4,
                current_batch = $5,
                total_batches = $6,
                stage_message = $7
            WHERE id = $8 AND status != 'CANCELLED'
            "#,
        )
        .bind(status)
        .bind(progress_pct)
        .bind(total_cues)
        .bind(translated_cues)
        .bind(current_batch)
        .bind(total_batches)
        .bind(stage_message)
        .bind(job.id)
        .execute(&self.db)
        .await?;

        self.broadcast_telemetry(
            job,
            status,
            progress_pct,
            total_cues,
            translated_cues,
            current_batch,
            total_batches,
            Some(stage_message),
            None,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn broadcast_telemetry(
        &self,
        job: &TranslationJob,
        status: &str,
        progress_pct: f32,
        total_cues: i32,
        translated_cues: i32,
        current_batch: i32,
        total_batches: i32,
        stage_message: Option<&str>,
        error_message: Option<&str>,
    ) {
        let file_name = Path::new(&job.source_file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "video.mkv".into());

        let telemetry = TranslationProgressTelemetry {
            job_id: job.id,
            file_name,
            target_language: job.target_language.clone(),
            gemini_model: job.gemini_model.clone(),
            status: status.to_string(),
            progress_pct,
            total_cues,
            translated_cues,
            current_batch,
            total_batches,
            stage_message: stage_message.map(|s| s.to_string()),
            error_message: error_message.map(|s| s.to_string()),
        };

        if let Ok(val) = serde_json::to_value(&telemetry) {
            self.ws.broadcast("translate_progress", val);
        }
    }

    async fn process_job(&self, job: &TranslationJob) -> Result<(), AppError> {
        let (api_key_opt, default_model) = resolve_gemini_settings(
            &self.sea_db,
            self.config.gemini_api_key.as_deref(),
            &self.config.gemini_model,
        )
        .await;

        let api_key = api_key_opt.ok_or_else(|| {
            AppError::BadRequest(
                "Google AI Studio API key is not configured. Add it in System Settings or GEMINI_API_KEY in backend/.env.".into(),
            )
        })?;

        let active_model = if job.gemini_model.trim().is_empty() {
            default_model
        } else {
            job.gemini_model.clone()
        };

        // Mark started_at
        sqlx::query("UPDATE translation_jobs SET started_at = NOW() WHERE id = $1")
            .bind(job.id)
            .execute(&self.db)
            .await?;

        // Stage 1: Inspect & Extract Subtitle Track
        self.update_stage(
            job,
            "EXTRACTING",
            5.0,
            0,
            0,
            0,
            0,
            "Inspecting subtitle streams with FFprobe...",
        )
        .await?;

        let tracks = probe_subtitle_tracks(&self.ffprobe_path, &job.source_file_path).await?;
        if tracks.is_empty() {
            return Err(AppError::BadRequest(
                "No embedded subtitle tracks found in source video file.".into(),
            ));
        }

        let selected_track = if job.source_stream_index >= 0 {
            tracks
                .iter()
                .find(|t| t.stream_index == job.source_stream_index)
                .cloned()
                .ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "Requested subtitle stream #{} was not found in video.",
                        job.source_stream_index
                    ))
                })?
        } else {
            pick_recommended_track(&tracks).cloned().ok_or_else(|| {
                AppError::BadRequest(
                    "No text-based subtitle tracks found (only bitmap/PGS/VobSub tracks exist).".into(),
                )
            })?
        };

        let sub_format = SubtitleFormat::from_codec(&selected_track.codec_name);
        let sub_ext = sub_format.extension();
        tokio::fs::create_dir_all(&self.work_dir).await?;

        let extracted_path = self.work_dir.join(format!("{}_source.{sub_ext}", job.id));
        let translated_path = self
            .work_dir
            .join(format!("{}_translated_{}.{sub_ext}", job.id, job.target_language_code));

        self.update_stage(
            job,
            "EXTRACTING",
            8.0,
            0,
            0,
            0,
            0,
            &format!(
                "Extracting {} subtitle stream #{}...",
                selected_track.codec_name.to_uppercase(),
                selected_track.stream_index
            ),
        )
        .await?;

        extract_subtitle(
            &self.ffmpeg_path,
            &job.source_file_path,
            selected_track.stream_index,
            &selected_track.codec_name,
            &extracted_path,
        )
        .await?;

        let extracted_str = extracted_path.to_string_lossy().to_string();
        let translated_str = translated_path.to_string_lossy().to_string();

        sqlx::query(
            r#"
            UPDATE translation_jobs
            SET
                source_stream_index = $1,
                source_sub_codec = $2,
                source_language = COALESCE(source_language, $3),
                extracted_sub_path = $4,
                translated_sub_path = $5
            WHERE id = $6
            "#,
        )
        .bind(selected_track.stream_index)
        .bind(&selected_track.codec_name)
        .bind(&selected_track.language)
        .bind(&extracted_str)
        .bind(&translated_str)
        .bind(job.id)
        .execute(&self.db)
        .await?;

        if self.is_cancelled(job.id).await {
            return Ok(());
        }

        // Stage 2: Parse & Translate Subtitle Cues via Gemini
        let raw_sub_content = tokio::fs::read_to_string(&extracted_path).await?;
        let parsed_sub = ParsedSubtitle::parse(&raw_sub_content, sub_format);
        let cues = parsed_sub.cues();

        if cues.is_empty() {
            return Err(AppError::BadRequest(
                "Extracted subtitle track contains no translatable dialogue lines.".into(),
            ));
        }

        let total_cues = cues.len() as i32;
        let chunks: Vec<&[crate::features::translator::subtitle::SubtitleCue]> =
            cues.chunks(TRANSLATION_BATCH_SIZE).collect();
        let total_batches = chunks.len() as i32;

        self.update_stage(
            job,
            "TRANSLATING",
            12.0,
            total_cues,
            0,
            0,
            total_batches,
            &format!(
                "Translating {total_cues} subtitle lines across {total_batches} batch(es) via {active_model}..."
            ),
        )
        .await?;

        let media_hint = Path::new(&job.source_file_path)
            .file_stem()
            .and_then(|s| s.to_str());
        let src_lang_hint = job
            .source_language
            .as_deref()
            .or(selected_track.language.as_deref());

        let mut all_translations: HashMap<usize, String> = HashMap::with_capacity(cues.len());
        let mut prev_context: Vec<(String, String)> = Vec::new();
        let mut translated_cues_count: i32 = 0;

        for (idx, batch) in chunks.iter().enumerate() {
            if self.is_cancelled(job.id).await {
                info!("Translation job {} was cancelled during translation", job.id);
                return Ok(());
            }

            let batch_num = (idx + 1) as i32;
            let batch_result = self
                .gemini
                .translate_batch(
                    &api_key,
                    &active_model,
                    src_lang_hint,
                    &job.target_language,
                    sub_format,
                    batch,
                    &prev_context,
                    media_hint,
                )
                .await?;

            // Collect context from the tail of this batch for dialogue continuity
            let mut next_context = Vec::new();
            for cue in *batch {
                if let Some(tr) = batch_result.get(&cue.id) {
                    all_translations.insert(cue.id, tr.clone());
                }
            }
            for cue in batch.iter().rev().take(3).rev() {
                if let Some(tr) = all_translations.get(&cue.id) {
                    next_context.push((cue.text.clone(), tr.clone()));
                }
            }
            prev_context = next_context;

            translated_cues_count += batch.len() as i32;
            let pct = 12.0 + (translated_cues_count as f32 / total_cues as f32) * 78.0;

            self.update_stage(
                job,
                "TRANSLATING",
                pct.min(90.0),
                total_cues,
                translated_cues_count,
                batch_num,
                total_batches,
                &format!(
                    "Translated batch {batch_num}/{total_batches} ({translated_cues_count}/{total_cues} lines)"
                ),
            )
            .await?;
        }

        if self.is_cancelled(job.id).await {
            return Ok(());
        }

        // Write translated subtitle file
        let translated_sub_content = parsed_sub.apply_translations(&all_translations);
        tokio::fs::write(&translated_path, translated_sub_content.as_bytes()).await?;

        // Stage 3: Embed translated subtitle back into video
        self.update_stage(
            job,
            "EMBEDDING",
            92.0,
            total_cues,
            translated_cues_count,
            total_batches,
            total_batches,
            "Muxing translated subtitle track into video container...",
        )
        .await?;

        embed_subtitle(
            &self.ffmpeg_path,
            &self.ffprobe_path,
            &job.source_file_path,
            &translated_path,
            &job.target_file_path,
            &job.target_language_code,
            &job.target_language,
            &job.embed_mode,
        )
        .await?;

        sqlx::query(
            r#"
            UPDATE translation_jobs
            SET
                status = 'COMPLETED',
                progress_pct = 100.0,
                total_cues = $1,
                translated_cues = $2,
                current_batch = $3,
                total_batches = $4,
                stage_message = 'Completed! Translated subtitle embedded into video.',
                completed_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(total_cues)
        .bind(translated_cues_count)
        .bind(total_batches)
        .bind(total_batches)
        .bind(job.id)
        .execute(&self.db)
        .await?;

        self.broadcast_telemetry(
            job,
            "COMPLETED",
            100.0,
            total_cues,
            translated_cues_count,
            total_batches,
            total_batches,
            Some("Completed! Translated subtitle embedded into video."),
            None,
        );

        info!("AI translation job {} completed successfully -> '{}'", job.id, job.target_file_path);
        Ok(())
    }
}
