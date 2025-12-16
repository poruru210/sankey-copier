use crate::ports::UpdateBroadcaster;
use async_trait::async_trait;
use tokio::sync::broadcast;

/// Implementation of UpdateBroadcaster using tokio broadcast channel
#[derive(Clone)]
pub struct WebsocketBroadcaster {
    tx: broadcast::Sender<String>,
}

impl WebsocketBroadcaster {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl UpdateBroadcaster for WebsocketBroadcaster {
    async fn broadcast_snapshot(&self) {
        // Snapshot logic might be handled differently or require additional dependencies
        // For now, we mainly use this for specific event notifications
    }

    async fn broadcast_ea_disconnected(&self, account_id: &str) {
        let _ = self.tx.send(format!("ea_disconnected:{}", account_id));
    }

    async fn broadcast_settings_updated(&self, json: &str) {
        let _ = self.tx.send(format!("settings_updated:{}", json));
    }
}
