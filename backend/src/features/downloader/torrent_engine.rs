use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use librqbit::{
    api::TorrentIdOrHash,
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
};
use sqlx::{PgPool, Row};
use tracing::{info, warn};
use uuid::Uuid;

type ManagedTorrentHandle = Arc<ManagedTorrent>;

use crate::core::error::AppError;
use crate::core::ws::WsHub;
use crate::features::downloader::models::{AddTorrentRequest, DownloadJob, TorrentStatsSnapshot};

pub struct TorrentEngine {
    pub session: Arc<Session>,
    pub db: PgPool,
    pub ws: WsHub,
    pub downloads_dir: PathBuf,
}

impl TorrentEngine {
    pub async fn new(
        downloads_dir: PathBuf,
        db: PgPool,
        ws: WsHub,
    ) -> Result<Arc<Self>, AppError> {
        tokio::fs::create_dir_all(&downloads_dir).await?;

        let session = Session::new_with_opts(
            downloads_dir.clone(),
            SessionOptions::default(),
        )
        .await
        .map_err(|e| AppError::Torrent(format!("Failed to initialize librqbit session: {}", e)))?;

        Ok(Arc::new(Self {
            session,
            db,
            ws,
            downloads_dir,
        }))
    }

    /// Add a new torrent by magnet URI or torrent URL
    pub async fn add_torrent(&self, req: AddTorrentRequest) -> Result<DownloadJob, AppError> {
        let magnet_uri = req.magnet_uri.trim().to_string();
        if !magnet_uri.starts_with("magnet:") && !magnet_uri.starts_with("http://") && !magnet_uri.starts_with("https://") {
            return Err(AppError::BadRequest("Invalid torrent source: must be a magnet link or http(s) URL".into()));
        }

        // Try extracting fallback name from magnet URI
        let mut fallback_name = extract_name_from_magnet(&magnet_uri)
            .unwrap_or_else(|| "Torrent Download".to_string());

        // If nyaa_id is given and name is generic, fetch title from nyaa_torrents
        if let Some(nyaa_id) = req.nyaa_id {
            if let Ok(Some(row)) = sqlx::query("SELECT title FROM nyaa_torrents WHERE id = $1")
                .bind(nyaa_id)
                .fetch_optional(&self.db)
                .await
            {
                if let Ok(title) = row.try_get::<String, _>("title") {
                    fallback_name = title;
                }
            }
        }

        let opts = AddTorrentOptions::default();
        let add_res = self
            .session
            .add_torrent(AddTorrent::from_url(&magnet_uri), Some(opts))
            .await
            .map_err(|e| AppError::Torrent(format!("Failed to add torrent to session: {}", e)))?;

        let handle: ManagedTorrentHandle = match add_res {
            AddTorrentResponse::Added(_id, h) => h,
            AddTorrentResponse::AlreadyManaged(_id, h) => h,
            _ => {
                return Err(AppError::Torrent("Unexpected response from torrent engine".into()));
            }
        };

        let info_hash = handle.info_hash().as_string();
        let resolved_name = handle
            .with_metadata(|m| m.info.name().map(|n| n.to_string()))
            .ok()
            .flatten()
            .unwrap_or(fallback_name);

        let total_bytes = handle.stats().total_bytes as i64;
        let save_path = self.downloads_dir.to_string_lossy().to_string();

        let insert_query = r#"
            INSERT INTO download_jobs (
                nyaa_torrent_id, info_hash, name, magnet_uri, save_path, status,
                total_bytes, downloaded_bytes, uploaded_bytes, download_speed, upload_speed,
                progress_pct, eta_seconds, seeders, peers, error_message
            ) VALUES ($1, $2, $3, $4, $5, 'DOWNLOADING', $6, 0, 0, 0, 0, 0.0, NULL, 0, 0, NULL)
            ON CONFLICT (info_hash) DO UPDATE SET
                status = 'DOWNLOADING',
                updated_at = NOW()
            RETURNING
                id, nyaa_torrent_id, info_hash, name, magnet_uri, save_path, status,
                total_bytes, downloaded_bytes, uploaded_bytes, download_speed, upload_speed,
                progress_pct, eta_seconds, seeders, peers, error_message, started_at,
                completed_at, created_at, updated_at
        "#;

        let job_result = sqlx::query_as::<_, DownloadJob>(insert_query)
            .bind(req.nyaa_id)
            .bind(&info_hash)
            .bind(&resolved_name)
            .bind(&magnet_uri)
            .bind(&save_path)
            .bind(total_bytes)
            .fetch_one(&self.db)
            .await;

        let job = match job_result {
            Ok(job) => job,
            Err(e) => {
                let is_fk_err = if let sqlx::Error::Database(ref db_err) = e {
                    db_err.code().as_deref() == Some("23503")
                        || db_err.message().contains("download_jobs_nyaa_torrent_id_fkey")
                } else {
                    false
                };

                if is_fk_err && req.nyaa_id.is_some() {
                    warn!(
                        "Foreign key constraint on nyaa_torrent_id failed for ID {:?}. Retrying insert with NULL nyaa_torrent_id...",
                        req.nyaa_id
                    );
                    sqlx::query_as::<_, DownloadJob>(insert_query)
                        .bind(Option::<i64>::None)
                        .bind(&info_hash)
                        .bind(&resolved_name)
                        .bind(&magnet_uri)
                        .bind(&save_path)
                        .bind(total_bytes)
                        .fetch_one(&self.db)
                        .await?
                } else {
                    return Err(e.into());
                }
            }
        };

        info!("Torrent added successfully: {} ({})", job.name, job.info_hash);
        Ok(job)
    }

    /// Pause a download job
    pub async fn pause_torrent(&self, id: Uuid) -> Result<(), AppError> {
        let job = self.get_job_by_id(id).await?;
        if let Some(handle) = self.find_handle_by_hash(&job.info_hash) {
            let _ = self.session.pause(&handle).await;
        }

        sqlx::query(
            "UPDATE download_jobs SET status = 'PAUSED', download_speed = 0, upload_speed = 0, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.db)
        .await?;

        info!("Torrent paused: {} ({})", job.name, id);
        Ok(())
    }

    /// Resume a paused download job
    pub async fn resume_torrent(&self, id: Uuid) -> Result<(), AppError> {
        let job = self.get_job_by_id(id).await?;
        if let Some(handle) = self.find_handle_by_hash(&job.info_hash) {
            let _ = self.session.unpause(&handle).await;
        }

        sqlx::query(
            "UPDATE download_jobs SET status = 'DOWNLOADING', updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.db)
        .await?;

        info!("Torrent resumed: {} ({})", job.name, id);
        Ok(())
    }

    /// Delete a torrent and optionally delete files
    pub async fn delete_torrent(&self, id: Uuid, delete_files: bool) -> Result<(), AppError> {
        let job = self.get_job_by_id(id).await?;
        if let Some(handle) = self.find_handle_by_hash(&job.info_hash) {
            let hash_id = handle.info_hash();
            if let Err(e) = self.session.delete(TorrentIdOrHash::Hash(hash_id), delete_files).await {
                warn!("Warning deleting torrent from session: {e}");
            }
        }

        sqlx::query("DELETE FROM download_jobs WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        info!("Torrent deleted: {} (delete_files: {})", job.name, delete_files);
        Ok(())
    }

    /// List all download jobs
    pub async fn list_jobs(&self) -> Result<Vec<DownloadJob>, AppError> {
        let jobs = sqlx::query_as::<_, DownloadJob>(
            r#"
            SELECT
                id, nyaa_torrent_id, info_hash, name, magnet_uri, save_path, status,
                total_bytes, downloaded_bytes, uploaded_bytes, download_speed, upload_speed,
                progress_pct, eta_seconds, seeders, peers, error_message, started_at,
                completed_at, created_at, updated_at
            FROM download_jobs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(jobs)
    }

    /// Start 1-second telemetry background loop
    pub fn start_monitor(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                let mut snapshots = Vec::new();

                // 1. Gather all active handles from librqbit
                let handles: Vec<ManagedTorrentHandle> = self.session.with_torrents(|torrents| {
                    let mut list = Vec::new();
                    for (_, h) in torrents {
                        list.push(h.clone());
                    }
                    list
                });

                for handle in handles {
                    let info_hash = handle.info_hash().as_string();
                    let stats = handle.stats();

                    let total_bytes = stats.total_bytes as i64;
                    let downloaded_bytes = stats.progress_bytes as i64;
                    let uploaded_bytes = stats.uploaded_bytes as i64;

                    let progress_pct = if total_bytes > 0 {
                        ((downloaded_bytes as f64 / total_bytes as f64) * 100.0) as f32
                    } else {
                        0.0
                    };

                    let (download_speed, upload_speed, peers, eta_seconds) = if let Some(live) = &stats.live {
                        let ds = live.download_speed.as_bytes() as i64;
                        let us = live.upload_speed.as_bytes() as i64;
                        let live_peers = live.snapshot.peer_stats.live as i32;
                        let eta = if ds > 0 && total_bytes > downloaded_bytes {
                            Some((total_bytes - downloaded_bytes) / ds)
                        } else {
                            None
                        };
                        (ds, us, live_peers, eta)
                    } else {
                        (0, 0, 0, None)
                    };

                    let is_completed = stats.finished || (total_bytes > 0 && downloaded_bytes >= total_bytes);

                    // Update DB record
                    let update_result = sqlx::query(
                        r#"
                        UPDATE download_jobs
                        SET
                            total_bytes = CASE WHEN $1 > 0 THEN $1 ELSE total_bytes END,
                            downloaded_bytes = $2,
                            uploaded_bytes = $3,
                            download_speed = $4,
                            upload_speed = $5,
                            progress_pct = $6,
                            eta_seconds = $7,
                            peers = $8,
                            status = CASE
                                WHEN status = 'PAUSED' THEN 'PAUSED'
                                WHEN $9 THEN 'COMPLETED'
                                ELSE 'DOWNLOADING'
                            END,
                            completed_at = CASE
                                WHEN $9 AND completed_at IS NULL THEN NOW()
                                ELSE completed_at
                            END,
                            updated_at = NOW()
                        WHERE info_hash = $10
                        RETURNING id, name, status
                        "#,
                    )
                    .bind(total_bytes)
                    .bind(downloaded_bytes)
                    .bind(uploaded_bytes)
                    .bind(download_speed)
                    .bind(upload_speed)
                    .bind(progress_pct)
                    .bind(eta_seconds)
                    .bind(peers)
                    .bind(is_completed)
                    .bind(&info_hash)
                    .fetch_optional(&self.db)
                    .await;

                    if let Ok(Some(row)) = update_result {
                        let id: Uuid = row.try_get("id").unwrap_or_default();
                        let name: String = row.try_get("name").unwrap_or_else(|_| "Unknown".into());
                        let status: String = row.try_get("status").unwrap_or_else(|_| "DOWNLOADING".into());

                        snapshots.push(TorrentStatsSnapshot {
                            id,
                            info_hash,
                            name,
                            status,
                            total_bytes,
                            downloaded_bytes,
                            uploaded_bytes,
                            download_speed,
                            upload_speed,
                            progress_pct,
                            eta_seconds,
                            seeders: 0,
                            peers,
                            error_message: None,
                        });
                    }
                }

                // 2. Broadcast snapshots via WebSocket
                if !snapshots.is_empty() {
                    if let Ok(val) = serde_json::to_value(&snapshots) {
                        self.ws.broadcast("torrent_progress", val);
                    }
                }
            }
        });
    }

    fn find_handle_by_hash(&self, hash_str: &str) -> Option<ManagedTorrentHandle> {
        let lower = hash_str.to_lowercase();
        self.session.with_torrents(|torrents| {
            for (_, h) in torrents {
                if h.info_hash().as_string().to_lowercase() == lower {
                    return Some(h.clone());
                }
            }
            None
        })
    }

    async fn get_job_by_id(&self, id: Uuid) -> Result<DownloadJob, AppError> {
        let job = sqlx::query_as::<_, DownloadJob>(
            r#"
            SELECT
                id, nyaa_torrent_id, info_hash, name, magnet_uri, save_path, status,
                total_bytes, downloaded_bytes, uploaded_bytes, download_speed, upload_speed,
                progress_pct, eta_seconds, seeders, peers, error_message, started_at,
                completed_at, created_at, updated_at
            FROM download_jobs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Download job with ID {} not found", id)))?;

        Ok(job)
    }
}

/// Helper function to parse display name from magnet link
fn extract_name_from_magnet(magnet: &str) -> Option<String> {
    for part in magnet.split('&') {
        if let Some(rest) = part.strip_prefix("dn=") {
            let decoded = urlencoding::decode(rest).ok()?;
            return Some(decoded.into_owned());
        }
    }
    None
}
