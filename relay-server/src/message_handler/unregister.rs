//! Unregister message handler
//!
//! Handles EA unregistration messages, updating connection status and notifying clients.

use super::MessageHandler;
use crate::models::UnregisterMessage;

impl MessageHandler {
    /// Handle EA unregistration
    pub(super) async fn handle_unregister(&self, msg: UnregisterMessage) {
        let account_id = &msg.account_id;
        self.connection_manager.unregister_ea(account_id).await;

        // Notify WebSocket clients
        let _ = self
            .broadcast_tx
            .send(format!("ea_disconnected:{}", account_id));
    }
}
