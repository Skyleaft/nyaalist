use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::features::downloader::models::{AddTorrentRequest, DeleteTorrentQuery, DownloadJob};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jobs", get(list_jobs))
        .route("/add", post(add_torrent))
        .route("/:id/pause", post(pause_torrent))
        .route("/:id/resume", post(resume_torrent))
        .route("/:id", delete(delete_torrent))
}

async fn list_jobs(
    State(state): State<AppState>,
) -> Result<Json<Vec<DownloadJob>>, AppError> {
    let jobs = state.torrent_engine.list_jobs().await?;
    Ok(Json(jobs))
}

async fn add_torrent(
    State(state): State<AppState>,
    Json(payload): Json<AddTorrentRequest>,
) -> Result<Json<DownloadJob>, AppError> {
    let job = state.torrent_engine.add_torrent(payload).await?;
    Ok(Json(job))
}

async fn pause_torrent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.torrent_engine.pause_torrent(id).await?;
    Ok(Json(json!({
        "status": "success",
        "message": "Torrent paused",
        "id": id
    })))
}

async fn resume_torrent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.torrent_engine.resume_torrent(id).await?;
    Ok(Json(json!({
        "status": "success",
        "message": "Torrent resumed",
        "id": id
    })))
}

async fn delete_torrent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<DeleteTorrentQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let delete_files = query.delete_files.unwrap_or(false);
    state.torrent_engine.delete_torrent(id, delete_files).await?;
    Ok(Json(json!({
        "status": "success",
        "message": "Torrent deleted",
        "id": id,
        "delete_files": delete_files
    })))
}

/// WebSocket Upgrade handler for live telemetry streaming
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_socket(socket, state))
}

async fn handle_ws_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws.tx.subscribe();

    info!("New WebSocket client connected for telemetry");

    // Task to forward broadcast messages from WsHub to the WebSocket client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json_text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json_text)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task to receive client messages (e.g. heartbeat pings)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_payload) => {
                    debug!("Received WS ping");
                }
                _ => {}
            }
        }
    });

    // If either task finishes, abort the other
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket client disconnected");
}
