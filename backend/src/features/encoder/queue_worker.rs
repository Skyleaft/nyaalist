use std::sync::Arc;
use std::time::Duration;
use sqlx::PgPool;
use tracing::{error, info};

use crate::core::ws::WsHub;
use crate::features::encoder::ffmpeg_runner::run_ffmpeg_job;
use crate::features::encoder::models::{EncodingJob, EncodingPreset};

pub struct EncodingQueueWorker {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub db: PgPool,
    pub ws: WsHub,
}

impl EncodingQueueWorker {
    pub fn new(
        ffmpeg_path: String,
        ffprobe_path: String,
        db: PgPool,
        ws: WsHub,
    ) -> Arc<Self> {
        Arc::new(Self {
            ffmpeg_path,
            ffprobe_path,
            db,
            ws,
        })
    }

    pub fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            info!("Starting Anime Video Transcoding Queue Worker...");
            loop {
                // 1. Fetch next queued job
                let next_job = sqlx::query_as::<_, EncodingJob>(
                    r#"
                    SELECT
                        id, source_file_path, target_file_path, preset_id, status,
                        progress_pct, current_frame, total_frames, fps, current_time_sec,
                        total_duration_sec, speed, source_size_bytes, encoded_size_bytes,
                        eta_seconds, error_message, created_at, started_at, completed_at
                    FROM encoding_jobs
                    WHERE status = 'QUEUED'
                    ORDER BY created_at ASC
                    LIMIT 1
                    "#
                )
                .fetch_optional(&self.db)
                .await;

                match next_job {
                    Ok(Some(job)) => {
                        // 2. Fetch associated preset
                        let preset_res = sqlx::query_as::<_, EncodingPreset>(
                            r#"
                            SELECT
                                id, name, description, video_codec, preset, crf,
                                target_resolution, audio_codec, audio_bitrate, container,
                                custom_ffmpeg_args, is_default, created_at
                            FROM encoding_presets
                            WHERE id = $1
                            "#
                        )
                        .bind(job.preset_id)
                        .fetch_optional(&self.db)
                        .await;

                        match preset_res {
                            Ok(Some(preset)) => {
                                info!(
                                    "Picked up encoding job {} for source '{}'",
                                    job.id, job.source_file_path
                                );
                                if let Err(e) = run_ffmpeg_job(
                                    &self.ffmpeg_path,
                                    &self.ffprobe_path,
                                    &job,
                                    &preset,
                                    &self.ws,
                                    &self.db,
                                ).await {
                                    error!("Error processing encode job {}: {}", job.id, e);
                                }
                            }
                            Ok(None) => {
                                error!(
                                    "Preset ID {} not found for job {}",
                                    job.preset_id, job.id
                                );
                                let _ = sqlx::query(
                                    "UPDATE encoding_jobs SET status = 'FAILED', error_message = 'Associated preset not found' WHERE id = $1"
                                )
                                .bind(job.id)
                                .execute(&self.db)
                                .await;
                            }
                            Err(e) => {
                                error!("Database error loading preset: {}", e);
                            }
                        }
                    }
                    Ok(None) => {
                        // No pending jobs, wait before next check
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        error!("Database error querying queue: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }
}
