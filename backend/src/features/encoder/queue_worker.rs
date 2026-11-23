use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::core::ws::WsHub;
use crate::features::encoder::ffmpeg_runner::run_ffmpeg_job;
use crate::features::encoder::models::{EncodeProgressTelemetry, EncodingJob, EncodingPreset};

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

            // Cleanup & recover any jobs previously stuck in ENCODING (e.g. from server restart or unspawnable process)
            let cleanup_res = sqlx::query(
                r#"
                UPDATE encoding_jobs
                SET
                    status = 'FAILED',
                    error_message = 'Transcode interrupted by backend restart or unspawnable process',
                    completed_at = NOW()
                WHERE status = 'ENCODING'
                "#
            )
            .execute(&self.db)
            .await;

            match cleanup_res {
                Ok(affected) => {
                    let count = affected.rows_affected();
                    if count > 0 {
                        warn!("Recovered and marked {} interrupted ENCODING job(s) as FAILED", count);
                    }
                }
                Err(e) => {
                    error!("Failed to recover stuck ENCODING jobs on startup: {}", e);
                }
            }

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
                                    let err_msg = e.to_string();
                                    error!("Error processing encode job {}: {}", job.id, err_msg);

                                    // Ensure database status is marked as FAILED if not already
                                    let _ = sqlx::query(
                                        r#"
                                        UPDATE encoding_jobs
                                        SET
                                            status = 'FAILED',
                                            error_message = $1,
                                            completed_at = NOW()
                                        WHERE id = $2 AND status != 'FAILED'
                                        "#
                                    )
                                    .bind(&err_msg)
                                    .bind(job.id)
                                    .execute(&self.db)
                                    .await;

                                    let filename = Path::new(&job.source_file_path)
                                        .file_name()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "video.mkv".into());

                                    let fail_telemetry = EncodeProgressTelemetry {
                                        job_id: job.id,
                                        file_name: filename,
                                        preset_name: preset.name.clone(),
                                        status: "FAILED".into(),
                                        progress_pct: 0.0,
                                        current_frame: 0,
                                        fps: 0.0,
                                        speed: 0.0,
                                        current_time_sec: 0.0,
                                        total_duration_sec: 0.0,
                                        eta_seconds: None,
                                        source_size_bytes: 0,
                                        current_size_bytes: 0,
                                    };
                                    if let Ok(val) = serde_json::to_value(&fail_telemetry) {
                                        self.ws.broadcast("encode_progress", val);
                                    }
                                }
                            }
                            Ok(None) => {
                                error!(
                                    "Preset ID {} not found for job {}",
                                    job.preset_id, job.id
                                );
                                let _ = sqlx::query(
                                    "UPDATE encoding_jobs SET status = 'FAILED', error_message = 'Associated preset not found', completed_at = NOW() WHERE id = $1"
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
