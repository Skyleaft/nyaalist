use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info, warn};

use crate::core::error::AppError;
use crate::core::ws::WsHub;
use crate::features::encoder::models::{EncodeProgressTelemetry, EncodingJob, EncodingPreset};

/// Attempts to resolve an FFmpeg or FFprobe binary path.
/// Checks exact path, .exe extension on Windows, PATH entries, and common installation locations.
pub fn resolve_binary_path(name_or_path: &str) -> String {
    let trimmed = name_or_path.trim();
    if trimmed.is_empty() {
        return name_or_path.to_string();
    }

    let p = Path::new(trimmed);
    // 1. Direct path check
    if p.is_file() {
        return trimmed.to_string();
    }
    #[cfg(windows)]
    {
        let p_exe = p.with_extension("exe");
        if p_exe.is_file() {
            return p_exe.to_string_lossy().to_string();
        }
    }

    // 2. If it doesn't contain path separators, search PATH
    let has_sep = trimmed.contains('/') || trimmed.contains('\\');
    if !has_sep {
        if let Ok(paths) = std::env::var("PATH") {
            for entry in std::env::split_paths(&paths) {
                let candidate = entry.join(trimmed);
                if candidate.is_file() {
                    return candidate.to_string_lossy().to_string();
                }
                #[cfg(windows)]
                {
                    let candidate_exe = entry.join(format!("{trimmed}.exe"));
                    if candidate_exe.is_file() {
                        return candidate_exe.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    // 3. Probe common installation paths on Windows
    #[cfg(windows)]
    {
        let exe_name = if trimmed.to_lowercase().ends_with(".exe") {
            trimmed.to_string()
        } else {
            format!("{trimmed}.exe")
        };

        let mut candidates: Vec<PathBuf> = Vec::new();

        // Relative project directories
        candidates.push(PathBuf::from(format!("./bin/{exe_name}")));
        candidates.push(PathBuf::from(format!("./tools/bin/{exe_name}")));
        candidates.push(PathBuf::from(format!("./backend/bin/{exe_name}")));

        // Standard root drives
        candidates.push(PathBuf::from(format!("C:\\ffmpeg\\bin\\{exe_name}")));
        candidates.push(PathBuf::from(format!("C:\\tools\\ffmpeg\\bin\\{exe_name}")));
        candidates.push(PathBuf::from(format!("C:\\tools\\bin\\{exe_name}")));
        candidates.push(PathBuf::from(format!("C:\\Program Files\\ffmpeg\\bin\\{exe_name}")));
        candidates.push(PathBuf::from(format!("C:\\Program Files (x86)\\ffmpeg\\bin\\{exe_name}")));
        candidates.push(PathBuf::from(format!("C:\\ProgramData\\chocolatey\\bin\\{exe_name}")));

        // WinGet / LocalAppData links and packages
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(Path::new(&local_app_data).join("Microsoft").join("WinGet").join("Links").join(&exe_name));
            let winget_pkg = Path::new(&local_app_data).join("Microsoft").join("WinGet").join("Packages");
            if winget_pkg.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&winget_pkg) {
                    for entry in entries.flatten() {
                        let bin_exe = entry.path().join("bin").join(&exe_name);
                        if bin_exe.is_file() {
                            return bin_exe.to_string_lossy().to_string();
                        }
                        if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                            for sub in sub_entries.flatten() {
                                let sub_bin_exe = sub.path().join("bin").join(&exe_name);
                                if sub_bin_exe.is_file() {
                                    return sub_bin_exe.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        // Scoop shims & apps
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            candidates.push(Path::new(&user_profile).join("scoop").join("shims").join(&exe_name));
            candidates.push(Path::new(&user_profile).join("scoop").join("apps").join("ffmpeg").join("current").join("bin").join(&exe_name));
        }

        for candidate in candidates {
            if candidate.is_file() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }

    trimmed.to_string()
}

/// Verifies whether the specified FFmpeg binary is executable and returns version banner.
pub async fn check_ffmpeg_available(ffmpeg_path: &str) -> Result<String, String> {
    let resolved = resolve_binary_path(ffmpeg_path);
    match Command::new(&resolved)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(out) => {
            if out.status.success() {
                let out_str = String::from_utf8_lossy(&out.stdout);
                let first_line = out_str.lines().next().unwrap_or("FFmpeg available").trim();
                Ok(first_line.to_string())
            } else {
                let err_str = String::from_utf8_lossy(&out.stderr);
                Err(format!("FFmpeg check returned non-zero exit status: {err_str}"))
            }
        }
        Err(e) => Err(format!("Could not spawn FFmpeg process ({e})")),
    }
}

/// Verifies whether the specified FFprobe binary is executable and returns version banner.
pub async fn check_ffprobe_available(ffprobe_path: &str) -> Result<String, String> {
    let resolved = resolve_binary_path(ffprobe_path);
    match Command::new(&resolved)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(out) => {
            if out.status.success() {
                let out_str = String::from_utf8_lossy(&out.stdout);
                let first_line = out_str.lines().next().unwrap_or("FFprobe available").trim();
                Ok(first_line.to_string())
            } else {
                let err_str = String::from_utf8_lossy(&out.stderr);
                Err(format!("FFprobe check returned non-zero exit status: {err_str}"))
            }
        }
        Err(e) => Err(format!("Could not spawn FFprobe process ({e})")),
    }
}

/// Probe video duration using ffprobe
pub async fn probe_duration(ffprobe_path: &str, file_path: &str) -> f32 {
    let resolved = resolve_binary_path(ffprobe_path);
    let output = match Command::new(&resolved)
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
            warn!("Could not run ffprobe ({}) on {}: {}", resolved, file_path, e);
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
    let resolved_ffmpeg = resolve_binary_path(ffmpeg_path);
    let resolved_ffprobe = resolve_binary_path(ffprobe_path);

    // 1. Ensure target parent dir exists
    let target_path = Path::new(&job.target_file_path);
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 2. Probe video duration
    let total_duration_sec = probe_duration(&resolved_ffprobe, &job.source_file_path).await;

    let source_size = tokio::fs::metadata(&job.source_file_path)
        .await
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let filename = Path::new(&job.source_file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video.mkv".into());

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

    info!(
        "Starting FFmpeg transcode for job {} ({}) using preset '{}'",
        job.id, filename, preset.name
    );

    let args = build_ffmpeg_args(job, preset);

    let mut child = match Command::new(&resolved_ffmpeg)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let spawn_err = format!("Failed to spawn FFmpeg process ({e}). Verify FFMPEG_BINARY_PATH or install FFmpeg.");
            error!("FFmpeg spawn error for job {}: {}", job.id, spawn_err);

            // Update job to FAILED in DB immediately
            let _ = sqlx::query(
                "UPDATE encoding_jobs SET status = 'FAILED', error_message = $1, completed_at = NOW() WHERE id = $2"
            )
            .bind(&spawn_err)
            .bind(job.id)
            .execute(db)
            .await;

            // Broadcast failed telemetry
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
                total_duration_sec,
                eta_seconds: None,
                source_size_bytes: source_size,
                current_size_bytes: 0,
            };
            if let Ok(val) = serde_json::to_value(&fail_telemetry) {
                ws.broadcast("encode_progress", val);
            }

            return Err(AppError::Internal(spawn_err));
        }
    };

    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::Internal("Failed to capture FFmpeg stdout pipe".into())
    })?;

    // Drain stderr concurrently to avoid OS pipe deadlock and capture error messages
    let stderr = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        if let Some(err) = stderr {
            let mut err_reader = BufReader::new(err).lines();
            let mut lines = Vec::new();
            while let Ok(Some(line)) = err_reader.next_line().await {
                lines.push(line);
                if lines.len() > 80 {
                    lines.remove(0);
                }
            }
            lines.join("\n")
        } else {
            String::new()
        }
    });

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
    let stderr_output = stderr_task.await.unwrap_or_default();

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
        let err_msg = if !stderr_output.trim().is_empty() {
            stderr_output
        } else {
            format!("FFmpeg failed with exit code {:?}", status.code())
        };

        error!("FFmpeg failed for job {}: {}", job.id, err_msg);

        let _ = sqlx::query("UPDATE encoding_jobs SET status = 'FAILED', error_message = $1, completed_at = NOW() WHERE id = $2")
            .bind(&err_msg)
            .bind(job.id)
            .execute(db)
            .await;

        let fail_telemetry = EncodeProgressTelemetry {
            job_id: job.id,
            file_name: filename,
            preset_name: preset.name.clone(),
            status: "FAILED".into(),
            progress_pct: 0.0,
            current_frame,
            fps: 0.0,
            speed: 0.0,
            current_time_sec: 0.0,
            total_duration_sec,
            eta_seconds: None,
            source_size_bytes: source_size,
            current_size_bytes: 0,
        };
        if let Ok(val) = serde_json::to_value(&fail_telemetry) {
            ws.broadcast("encode_progress", val);
        }

        Err(AppError::Internal(format!("FFmpeg failed: {err_msg}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_resolve_binary_path_fallback() {
        let path = resolve_binary_path("non_existent_ffmpeg_bin_12345");
        assert_eq!(path, "non_existent_ffmpeg_bin_12345");
    }

    #[test]
    fn test_resolve_binary_path_existing_file() {
        let manifest = "Cargo.toml";
        let resolved = resolve_binary_path(manifest);
        assert_eq!(resolved, manifest);
    }

    #[test]
    fn test_build_ffmpeg_args() {
        let job = EncodingJob {
            id: Uuid::new_v4(),
            source_file_path: "input.mkv".into(),
            target_file_path: "output.mkv".into(),
            preset_id: Uuid::new_v4(),
            status: "QUEUED".into(),
            progress_pct: 0.0,
            current_frame: 0,
            total_frames: None,
            fps: 0.0,
            current_time_sec: 0.0,
            total_duration_sec: 0.0,
            speed: 0.0,
            source_size_bytes: 100,
            encoded_size_bytes: 0,
            eta_seconds: None,
            error_message: None,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
        };

        let preset = EncodingPreset {
            id: Uuid::new_v4(),
            name: "720p Balanced".into(),
            description: None,
            video_codec: "libx265".into(),
            preset: "medium".into(),
            crf: 24,
            target_resolution: Some("1280x720".into()),
            audio_codec: "copy".into(),
            audio_bitrate: None,
            container: "mkv".into(),
            custom_ffmpeg_args: Some("-map 0".into()),
            is_default: true,
            created_at: chrono::Utc::now(),
        };

        let args = build_ffmpeg_args(&job, &preset);
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"input.mkv".to_string()));
        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx265".to_string()));
        assert!(args.contains(&"-vf".to_string()));
        assert!(args.contains(&"scale=1280:720".to_string()));
        assert!(args.contains(&"-map".to_string()));
        assert!(args.contains(&"0".to_string()));
        assert_eq!(args.last().unwrap(), "output.mkv");
    }
}
