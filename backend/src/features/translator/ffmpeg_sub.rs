use std::path::Path;
use std::process::Stdio;
use serde::Deserialize;
use tokio::process::Command;

use crate::core::error::AppError;
use crate::features::encoder::ffmpeg_runner::resolve_binary_path;
use crate::features::translator::models::SubtitleTrack;
use crate::features::translator::subtitle::SubtitleFormat;

#[derive(Debug, Deserialize)]
struct FfprobeStreamsOutput {
    streams: Option<Vec<FfprobeStream>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: i32,
    codec_name: Option<String>,
    disposition: Option<FfprobeDisposition>,
    tags: Option<FfprobeTags>,
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    default: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeTags {
    language: Option<String>,
    title: Option<String>,
}

pub fn is_text_subtitle_codec(codec: &str) -> bool {
    matches!(
        codec.to_lowercase().as_str(),
        "ass" | "ssa" | "subrip" | "srt" | "webvtt" | "mov_text" | "text" | "tx3g"
    )
}

/// Probes all embedded subtitle streams in a video file using ffprobe.
pub async fn probe_subtitle_tracks(
    ffprobe_path: &str,
    video_path: &str,
) -> Result<Vec<SubtitleTrack>, AppError> {
    if !Path::new(video_path).is_file() {
        return Err(AppError::NotFound(format!(
            "Source video file not found: {video_path}"
        )));
    }

    let resolved = resolve_binary_path(ffprobe_path);
    let output = Command::new(&resolved)
        .args([
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=index,codec_name:stream_tags=language,title:stream_disposition=default",
            "-of",
            "json",
            video_path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            AppError::BadRequest(format!(
                "Failed to spawn FFprobe ({resolved}): {e}. Verify FFPROBE_BINARY_PATH."
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::BadRequest(format!(
            "FFprobe failed inspecting subtitle tracks: {}",
            stderr.trim()
        )));
    }

    let parsed: FfprobeStreamsOutput = serde_json::from_slice(&output.stdout).map_err(|e| {
        AppError::Internal(format!("Failed to parse FFprobe subtitle JSON: {e}"))
    })?;

    let streams = parsed.streams.unwrap_or_default();
    let tracks = streams
        .into_iter()
        .enumerate()
        .map(|(sub_idx, s)| {
            let codec_name = s.codec_name.unwrap_or_else(|| "unknown".into());
            let is_text_based = is_text_subtitle_codec(&codec_name);
            let is_default = s.disposition.and_then(|d| d.default).unwrap_or(0) == 1;
            let language = s.tags.as_ref().and_then(|t| t.language.clone());
            let title = s.tags.as_ref().and_then(|t| t.title.clone());
            SubtitleTrack {
                stream_index: s.index,
                sub_index: sub_idx as i32,
                codec_name,
                language,
                title,
                is_text_based,
                is_default,
            }
        })
        .collect();

    Ok(tracks)
}

/// Picks the best text-based subtitle track (preferring full English dialogue).
pub fn pick_recommended_track(tracks: &[SubtitleTrack]) -> Option<&SubtitleTrack> {
    let text_tracks: Vec<&SubtitleTrack> = tracks.iter().filter(|t| t.is_text_based).collect();
    if text_tracks.is_empty() {
        return None;
    }

    let is_english = |t: &&SubtitleTrack| {
        let lang_match = t
            .language
            .as_deref()
            .is_some_and(|l| matches!(l.to_lowercase().as_str(), "eng" | "en" | "english"));
        let title_match = t
            .title
            .as_deref()
            .is_some_and(|title| title.to_lowercase().contains("english"));
        lang_match || title_match
    };

    let is_signs_only = |t: &&SubtitleTrack| {
        t.title.as_deref().is_some_and(|title| {
            let lower = title.to_lowercase();
            lower.contains("sign") || lower.contains("song")
        })
    };

    text_tracks
        .iter()
        .copied()
        .find(|t| is_english(t) && !is_signs_only(t))
        .or_else(|| text_tracks.iter().copied().find(is_english))
        .or_else(|| text_tracks.iter().copied().find(|t| t.is_default))
        .or_else(|| text_tracks.first().copied())
}

/// Extracts the specified subtitle stream from the video to an ASS or SRT file.
pub async fn extract_subtitle(
    ffmpeg_path: &str,
    video_path: &str,
    stream_index: i32,
    codec_name: &str,
    out_path: &Path,
) -> Result<SubtitleFormat, AppError> {
    if !is_text_subtitle_codec(codec_name) {
        return Err(AppError::BadRequest(format!(
            "Subtitle stream #{stream_index} uses image-based codec '{codec_name}', which cannot be extracted as text for AI translation."
        )));
    }

    if let Some(parent) = out_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let format = SubtitleFormat::from_codec(codec_name);
    let sub_codec_arg = match format {
        SubtitleFormat::Ass => "ass",
        SubtitleFormat::Srt => "srt",
    };

    let resolved = resolve_binary_path(ffmpeg_path);
    let map_spec = format!("0:{stream_index}");
    let out_str = out_path.to_string_lossy().to_string();

    let output = Command::new(&resolved)
        .args([
            "-y",
            "-v",
            "error",
            "-i",
            video_path,
            "-map",
            &map_spec,
            "-c:s",
            sub_codec_arg,
            &out_str,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to spawn FFmpeg for extraction: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!(
            "FFmpeg failed extracting subtitle stream #{stream_index}: {}",
            stderr.trim()
        )));
    }

    Ok(format)
}

/// Muxes the translated subtitle file back into the video container without re-encoding video/audio.
pub async fn embed_subtitle(
    ffmpeg_path: &str,
    ffprobe_path: &str,
    source_video: &str,
    translated_sub: &Path,
    target_video: &str,
    target_lang_code: &str,
    target_lang_name: &str,
    embed_mode: &str,
) -> Result<(), AppError> {
    let target_path = Path::new(target_video);
    if let Some(parent) = target_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let existing_subs = probe_subtitle_tracks(ffprobe_path, source_video)
        .await
        .map(|v| v.len())
        .unwrap_or(0);

    let ext = target_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mkv")
        .to_lowercase();
    let is_mp4_family = matches!(ext.as_str(), "mp4" | "m4v" | "mov");
    let sub_codec = if is_mp4_family { "mov_text" } else { "copy" };

    // Always write to a temp sibling file first so even in-place replacement is atomic and safe.
    let parent_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_name = format!(
        ".nyaalist_mux_{}.{ext}",
        uuid::Uuid::new_v4()
    );
    let tmp_path = parent_dir.join(tmp_name);
    let tmp_str = tmp_path.to_string_lossy().to_string();

    let sub_str = translated_sub.to_string_lossy().to_string();
    let title_meta = format!("{target_lang_name} (AI)");

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        source_video.into(),
        "-i".into(),
        sub_str,
    ];

    if embed_mode == "mux_replace" {
        args.extend([
            "-map".into(),
            "0:v".into(),
            "-map".into(),
            "0:a?".into(),
            "-map".into(),
            "0:t?".into(),
            "-map".into(),
            "1:0".into(),
            "-c:v".into(),
            "copy".into(),
            "-c:a".into(),
            "copy".into(),
            "-c:t".into(),
            "copy".into(),
            "-c:s".into(),
            sub_codec.into(),
            "-disposition:s:0".into(),
            "default".into(),
            "-metadata:s:s:0".into(),
            format!("language={target_lang_code}"),
            "-metadata:s:s:0".into(),
            format!("title={title_meta}"),
        ]);
    } else {
        let new_sub_idx = existing_subs;
        args.extend([
            "-map".into(),
            "0".into(),
            "-map".into(),
            "1:0".into(),
            "-c:v".into(),
            "copy".into(),
            "-c:a".into(),
            "copy".into(),
            "-c:s".into(),
            sub_codec.into(),
            "-disposition:s".into(),
            "0".into(),
            format!("-disposition:s:{new_sub_idx}"),
            "default".into(),
            format!("-metadata:s:s:{new_sub_idx}"),
            format!("language={target_lang_code}"),
            format!("-metadata:s:s:{new_sub_idx}"),
            format!("title={title_meta}"),
        ]);
    }

    args.push(tmp_str.clone());

    let resolved = resolve_binary_path(ffmpeg_path);
    let output = Command::new(&resolved)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to spawn FFmpeg for embedding: {e}")))?;

    if !output.status.success() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!(
            "FFmpeg failed embedding translated subtitle: {}",
            stderr.trim()
        )));
    }

    if let Err(e) = tokio::fs::rename(&tmp_path, target_path).await {
        // Fallback copy+remove across filesystems if rename fails
        if let Err(copy_err) = tokio::fs::copy(&tmp_path, target_path).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(AppError::Internal(format!(
                "Failed moving muxed video to target '{target_video}': {e} / {copy_err}"
            )));
        }
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_recommended_track_prefers_english_dialogue() {
        let tracks = vec![
            SubtitleTrack {
                stream_index: 2,
                sub_index: 0,
                codec_name: "ass".into(),
                language: Some("eng".into()),
                title: Some("Signs & Songs".into()),
                is_text_based: true,
                is_default: false,
            },
            SubtitleTrack {
                stream_index: 3,
                sub_index: 1,
                codec_name: "ass".into(),
                language: Some("eng".into()),
                title: Some("English (Full)".into()),
                is_text_based: true,
                is_default: true,
            },
            SubtitleTrack {
                stream_index: 4,
                sub_index: 2,
                codec_name: "hdmv_pgs_subtitle".into(),
                language: Some("jpn".into()),
                title: None,
                is_text_based: false,
                is_default: false,
            },
        ];

        let rec = pick_recommended_track(&tracks).unwrap();
        assert_eq!(rec.stream_index, 3);
    }

    #[test]
    fn test_pick_recommended_track_ignores_bitmap_only() {
        let tracks = vec![SubtitleTrack {
            stream_index: 2,
            sub_index: 0,
            codec_name: "hdmv_pgs_subtitle".into(),
            language: Some("eng".into()),
            title: None,
            is_text_based: false,
            is_default: true,
        }];
        assert!(pick_recommended_track(&tracks).is_none());
    }
}
