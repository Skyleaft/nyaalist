use std::path::{Path, PathBuf};
use sqlx::{PgPool, Row};
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::core::error::AppError;
use crate::features::explore::title_parser::parse_release_title;
use crate::features::library::models::{LibraryFile, ScanDirectoryResult};

const DEFAULT_ORGANIZER_TEMPLATE: &str =
    "{anime_title}/Season {season}/{anime_title} - S{season:02d}E{episode:02d} [{resolution}].{ext}";

/// Checks whether a given path is an anime media video file
pub fn is_media_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "mkv" | "mp4" | "avi" | "webm" | "mov" | "m4v" | "flv"
    )
}

/// Sanitizes strings for safe cross-platform filesystem paths
pub fn sanitize_filename_part(part: &str) -> String {
    part.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Formats template string into target organized path
pub fn render_template(
    template: &str,
    anime_title: &str,
    season: i32,
    episode: &str,
    resolution: &str,
    ext: &str,
) -> String {
    let clean_title = sanitize_filename_part(anime_title);
    let clean_res = if resolution.is_empty() { "1080p" } else { resolution };
    let ep_num: Option<i32> = episode.parse().ok();

    let ep_formatted = match ep_num {
        Some(n) => format!("{:02}", n),
        None => episode.to_string(),
    };

    let s_02d = format!("{:02}", season);

    let mut result = template.to_string();
    result = result.replace("{anime_title}", &clean_title);
    result = result.replace("{season}", &season.to_string());
    result = result.replace("{season:02d}", &s_02d);
    result = result.replace("{episode:02d}", &ep_formatted);
    result = result.replace("{episode}", episode);
    result = result.replace("{resolution}", clean_res);
    result = result.replace("{ext}", ext);

    result
}

/// Moves or copies file across directories or filesystem mounts
pub async fn move_or_copy_file(source: &Path, destination: &Path) -> Result<(), AppError> {
    if !source.exists() {
        return Err(AppError::NotFound(format!(
            "Source file does not exist: {}",
            source.display()
        )));
    }

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Try fast atomic rename first
    if let Err(rename_err) = tokio::fs::rename(source, destination).await {
        warn!(
            "Atomic rename failed ({}), attempting cross-device copy...",
            rename_err
        );
        tokio::fs::copy(source, destination).await?;
        tokio::fs::remove_file(source).await?;
    }

    info!("Moved file from {} to {}", source.display(), destination.display());
    Ok(())
}

/// Scans the downloads directory and synchronizes newly found media files into the database
pub async fn scan_and_sync_downloads(
    downloads_dir: &Path,
    library_dir: &Path,
    db: &PgPool,
) -> Result<ScanDirectoryResult, AppError> {
    if !downloads_dir.exists() {
        tokio::fs::create_dir_all(downloads_dir).await?;
    }

    let mut scanned_count = 0;
    let mut new_files_count = 0;

    for entry in WalkDir::new(downloads_dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && is_media_file(path) {
            scanned_count += 1;
            let current_path = path.to_string_lossy().to_string();
            let original_filename = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".into());

            // Check if already tracked
            let existing = sqlx::query("SELECT id FROM library_files WHERE current_path = $1 OR original_filename = $2")
                .bind(&current_path)
                .bind(&original_filename)
                .fetch_optional(db)
                .await?;

            if existing.is_none() {
                // Parse file metadata using title parser
                let parsed = parse_release_title(&original_filename);
                let anime_title = if !parsed.anime_title.is_empty() {
                    parsed.anime_title
                } else {
                    path.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown Anime".into())
                };

                let episode = parsed.episode.unwrap_or_else(|| "01".into());
                let resolution = parsed.resolution.unwrap_or_else(|| "1080p".into());
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mkv");

                let meta_len = entry.metadata().map(|m| m.len() as i64).unwrap_or(0);

                // Attempt to match AniList metadata id
                let anilist_id: Option<i32> = sqlx::query(
                    r#"
                    SELECT anilist_id FROM title_mappings WHERE raw_title_cleaned = $1
                    UNION ALL
                    SELECT id AS anilist_id FROM anime_metadata WHERE title_romaji ILIKE $1 OR title_english ILIKE $1
                    LIMIT 1
                    "#
                )
                .bind(&anime_title)
                .fetch_optional(db)
                .await?
                .and_then(|row| row.try_get("anilist_id").ok());

                // Generate target library path recommendation
                let rel_path = render_template(
                    DEFAULT_ORGANIZER_TEMPLATE,
                    &anime_title,
                    1,
                    &episode,
                    &resolution,
                    ext,
                );
                let target_library_path = library_dir.join(&rel_path).to_string_lossy().to_string();

                // Attempt to link to download_jobs
                let download_job_id: Option<Uuid> = sqlx::query(
                    "SELECT id FROM download_jobs WHERE name ILIKE '%' || $1 || '%' OR $2 ILIKE '%' || name || '%' LIMIT 1"
                )
                .bind(&anime_title)
                .bind(&original_filename)
                .fetch_optional(db)
                .await?
                .and_then(|row| row.try_get("id").ok());

                sqlx::query(
                    r#"
                    INSERT INTO library_files (
                        download_job_id, anilist_id, original_filename, current_path,
                        target_library_path, episode, season, resolution, file_size_bytes,
                        is_moved, is_encoded
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, FALSE, FALSE)
                    "#
                )
                .bind(download_job_id)
                .bind(anilist_id)
                .bind(&original_filename)
                .bind(&current_path)
                .bind(&target_library_path)
                .bind(&episode)
                .bind(1) // default season 1
                .bind(&resolution)
                .bind(meta_len)
                .execute(db)
                .await?;

                new_files_count += 1;
            }
        }
    }

    let files = sqlx::query_as::<_, LibraryFile>(
        r#"
        SELECT
            id, download_job_id, anilist_id, original_filename, current_path,
            target_library_path, episode, season, resolution, file_size_bytes,
            is_moved, is_encoded, created_at, updated_at
        FROM library_files
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(db)
    .await?;

    info!(
        "Library scanner finished: {} files found, {} newly registered",
        scanned_count, new_files_count
    );

    Ok(ScanDirectoryResult {
        scanned_count,
        new_files_count,
        files,
    })
}

/// Automatically organizes a single library file according to the template
pub async fn auto_organize_file(
    file_id: Uuid,
    custom_template: Option<String>,
    library_dir: &Path,
    db: &PgPool,
) -> Result<LibraryFile, AppError> {
    let file = sqlx::query_as::<_, LibraryFile>(
        r#"
        SELECT
            id, download_job_id, anilist_id, original_filename, current_path,
            target_library_path, episode, season, resolution, file_size_bytes,
            is_moved, is_encoded, created_at, updated_at
        FROM library_files
        WHERE id = $1
        "#
    )
    .bind(file_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Library file with ID {} not found", file_id)))?;

    // Determine anime title from AniList metadata if available, else from filename
    let anime_title: String = if let Some(anilist_id) = file.anilist_id {
        sqlx::query("SELECT COALESCE(title_english, title_romaji) AS title FROM anime_metadata WHERE id = $1")
            .bind(anilist_id)
            .fetch_optional(db)
            .await?
            .and_then(|row| row.try_get("title").ok())
            .unwrap_or_else(|| {
                parse_release_title(&file.original_filename).anime_title
            })
    } else {
        let parsed = parse_release_title(&file.original_filename);
        if !parsed.anime_title.is_empty() {
            parsed.anime_title
        } else {
            Path::new(&file.original_filename)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown Anime".into())
        }
    };

    let season = file.season.unwrap_or(1);
    let episode = file.episode.as_deref().unwrap_or("01");
    let resolution = file.resolution.as_deref().unwrap_or("1080p");
    let ext = Path::new(&file.original_filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mkv");

    let template = custom_template.as_deref().unwrap_or(DEFAULT_ORGANIZER_TEMPLATE);
    let rel_target = render_template(template, &anime_title, season, episode, resolution, ext);
    let full_target = library_dir.join(&rel_target);

    // Perform move
    let source_path = PathBuf::from(&file.current_path);
    move_or_copy_file(&source_path, &full_target).await?;

    let final_target_str = full_target.to_string_lossy().to_string();

    let updated_file = sqlx::query_as::<_, LibraryFile>(
        r#"
        UPDATE library_files
        SET
            current_path = $1,
            target_library_path = $1,
            is_moved = TRUE,
            updated_at = NOW()
        WHERE id = $2
        RETURNING
            id, download_job_id, anilist_id, original_filename, current_path,
            target_library_path, episode, season, resolution, file_size_bytes,
            is_moved, is_encoded, created_at, updated_at
        "#
    )
    .bind(&final_target_str)
    .bind(file_id)
    .fetch_one(db)
    .await?;

    Ok(updated_file)
}
