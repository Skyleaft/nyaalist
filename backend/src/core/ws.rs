use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub event: String,
    pub data: serde_json::Value,
}

#[derive(Clone)]
pub struct WsHub {
    pub tx: broadcast::Sender<WsMessage>,
}

impl WsHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn broadcast(&self, event: &str, data: serde_json::Value) {
        let msg = WsMessage {
            event: event.to_string(),
            data,
        };
        let _ = self.tx.send(msg);
    }
}
