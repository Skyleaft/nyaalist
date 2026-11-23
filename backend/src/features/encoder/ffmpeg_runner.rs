use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::core::error::AppError;
use crate::core::ws::WsHub;
use crate::features::encoder::models::{EncodeProgressTelemetry, EncodingJob, EncodingPreset};

/// Probe video duration using ffprobe
pub async fn probe_duration(ffprobe_path: &str, file_path: &str) -> f32 {
    let output = match Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .await
    {
        Ok(out) => out,
        Err(e) => {
            warn!("Could not run ffprobe on {}: {}", file_path, e);
            return 0.0;
        }
    };

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        text.trim().parse::<f32>().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Constructs FFmpeg arguments from job and preset
pub fn build_ffmpeg_args(job: &EncodingJob, preset: &EncodingPreset) -> Vec<String> {
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        job.source_file_path.clone(),
        "-c:v".into(),
        preset.video_codec.clone(),
        "-preset".into(),
        preset.preset.clone(),
        "-crf".into(),
        preset.crf.to_string(),
    ];

    if let Some(res) = &preset.target_resolution {
        if !res.is_empty() {
            let scale = res.replace('x', ":");
            args.push("-vf".into());
            args.push(format!("scale={scale}"));
        }
    }

    args.push("-c:a".into());
    args.push(preset.audio_codec.clone());

    if let Some(ab) = &preset.audio_bitrate {
        if !ab.is_empty() && preset.audio_codec != "copy" {
            args.push("-b:a".into());
            args.push(ab.clone());
        }
    }

    args.push("-c:s".into());
    args.push("copy".into());

    if let Some(custom) = &preset.custom_ffmpeg_args {
        if !custom.trim().is_empty() {
            for arg in custom.split_whitespace() {
                args.push(arg.into());
            }
        }
    }

    args.push("-progress".into());
    args.push("pipe:1".into());
    args.push("-nostats".into());
    args.push(job.target_file_path.clone());

    args
}

/// Executes FFmpeg job with real-time progress parsing and WebSocket broadcast
pub async fn run_ffmpeg_job(
    ffmpeg_path: &str,
    ffprobe_path: &str,
    job: &EncodingJob,
    preset: &EncodingPreset,
    ws: &WsHub,
    db: &PgPool,
) -> Result<(), AppError> {
    // 1. Ensure target parent dir exists
    let target_path = Path::new(&job.target_file_path);
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 2. Probe video duration
    let total_duration_sec = probe_duration(ffprobe_path, &job.source_file_path).await;

    let source_size = tokio::fs::metadata(&job.source_file_path)
        .await
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // 3. Mark job as ENCODING
    sqlx::query(
        r#"
        UPDATE encoding_jobs
        SET
            status = 'ENCODING',
            total_duration_sec = $1,
            source_size_bytes = $2,
            started_at = NOW()
        WHERE id = $3
        "#
    )
    .bind(total_duration_sec)
    .bind(source_size)
    .bind(job.id)
    .execute(db)
    .await?;

    let filename = Path::new(&job.source_file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video.mkv".into());

    info!(
        "Starting FFmpeg transcode for job {} ({}) using preset '{}'",
        job.id, filename, preset.name
    );

    let args = build_ffmpeg_args(job, preset);

    let mut child = Command::new(ffmpeg_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            AppError::Internal(format!("Failed to spawn FFmpeg process ({e}). Verify FFMPEG_BINARY_PATH."))
        })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::Internal("Failed to capture FFmpeg stdout pipe".into())
    })?;

    let mut reader = BufReader::new(stdout).lines();

    let mut current_frame: i64 = 0;
    let mut fps: f32 = 0.0;
    let mut current_time_sec: f32 = 0.0;
    let mut speed: f32 = 0.0;
    let mut encoded_size_bytes: i64 = 0;
    let mut last_broadcast = Instant::now();
    let broadcast_interval = Duration::from_millis(500);

    while let Ok(Some(line)) = reader.next_line().await {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "frame" => {
                    current_frame = v.parse().unwrap_or(current_frame);
                }
                "fps" => {
                    fps = v.parse().unwrap_or(fps);
                }
                "out_time_us" => {
                    if let Ok(us) = v.parse::<f32>() {
                        current_time_sec = us / 1_000_000.0;
                    }
                }
                "total_size" => {
                    encoded_size_bytes = v.parse().unwrap_or(encoded_size_bytes);
                }
                "speed" => {
                    let s_clean = v.replace('x', "");
                    speed = s_clean.parse().unwrap_or(speed);
                }
                "progress" => {
                    let progress_pct = if total_duration_sec > 0.0 {
                        ((current_time_sec / total_duration_sec) * 100.0).clamp(0.0, 99.9)
                    } else {
                        0.0
                    };

                    let eta_seconds = if speed > 0.0 && total_duration_sec > current_time_sec {
                        Some(((total_duration_sec - current_time_sec) / speed) as i64)
                    } else {
                        None
                    };

                    if last_broadcast.elapsed() >= broadcast_interval {
                        last_broadcast = Instant::now();

                        // 1. Broadcast WS frame
                        let telemetry = EncodeProgressTelemetry {
                            job_id: job.id,
                            file_name: filename.clone(),
                            preset_name: preset.name.clone(),
                            status: "ENCODING".into(),
                            progress_pct,
                            current_frame,
                            fps,
                            speed,
                            current_time_sec,
                            total_duration_sec,
                            eta_seconds,
                            source_size_bytes: source_size,
                            current_size_bytes: encoded_size_bytes,
                        };

                        if let Ok(val) = serde_json::to_value(&telemetry) {
                            ws.broadcast("encode_progress", val);
                        }

                        // 2. Persist progress into DB
                        let _ = sqlx::query(
                            r#"
                            UPDATE encoding_jobs
                            SET
                                progress_pct = $1,
                                current_frame = $2,
                                fps = $3,
                                current_time_sec = $4,
                                speed = $5,
                                encoded_size_bytes = $6,
                                eta_seconds = $7
                            WHERE id = $8
                            "#
                        )
                        .bind(progress_pct)
                        .bind(current_frame)
                        .bind(fps)
                        .bind(current_time_sec)
                        .bind(speed)
                        .bind(encoded_size_bytes)
                        .bind(eta_seconds)
                        .bind(job.id)
                        .execute(db)
                        .await;
                    }
                }
                _ => {}
            }
        }
    }

    let status = child.wait().await?;
    if status.success() {
        let final_size = tokio::fs::metadata(&job.target_file_path)
            .await
            .map(|m| m.len() as i64)
            .unwrap_or(encoded_size_bytes);

        sqlx::query(
            r#"
            UPDATE encoding_jobs
            SET
                status = 'COMPLETED',
                progress_pct = 100.0,
                current_time_sec = total_duration_sec,
                encoded_size_bytes = $1,
                completed_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(final_size)
        .bind(job.id)
        .execute(db)
        .await?;

        // Link with library_files if source or target matches
        let _ = sqlx::query(
            "UPDATE library_files SET is_encoded = TRUE WHERE current_path = $1 OR original_filename = $2"
        )
        .bind(&job.source_file_path)
        .bind(&filename)
        .execute(db)
        .await;

        let final_telemetry = EncodeProgressTelemetry {
            job_id: job.id,
            file_name: filename,
            preset_name: preset.name.clone(),
            status: "COMPLETED".into(),
            progress_pct: 100.0,
            current_frame,
            fps,
            speed,
            current_time_sec: total_duration_sec,
            total_duration_sec,
            eta_seconds: Some(0),
            source_size_bytes: source_size,
            current_size_bytes: final_size,
        };

        if let Ok(val) = serde_json::to_value(&final_telemetry) {
            ws.broadcast("encode_progress", val);
        }

        info!("FFmpeg transcode completed successfully for job {}", job.id);
        Ok(())
    } else {
        let stderr_output = child.wait_with_output().await;
        let err_msg = stderr_output
            .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
            .unwrap_or_else(|_| "FFmpeg failed with non-zero exit code".into());

        error!("FFmpeg failed for job {}: {}", job.id, err_msg);

        sqlx::query("UPDATE encoding_jobs SET status = 'FAILED', error_message = $1 WHERE id = $2")
            .bind(&err_msg)
            .bind(job.id)
            .execute(db)
            .await?;

        Err(AppError::Internal(format!("FFmpeg failed: {err_msg}")))
    }
}
