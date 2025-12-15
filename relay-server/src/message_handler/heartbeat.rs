//! Heartbeat message handler
//!
//! Handles heartbeat messages by delegating to StatusService.
//!
//! (Delegated to StatusService logic:
//!  - Master: DISABLED (web_ui OFF or !is_trade_allowed) or CONNECTED
//!  - Slave: DISABLED (web_ui OFF or !is_trade_allowed) or ENABLED (master not connected) or CONNECTED)

use super::MessageHandler;
use crate::models::HeartbeatMessage;

impl MessageHandler {
    /// Handle heartbeat messages
    /// Delegates logic to StatusService
    pub(super) async fn handle_heartbeat(&self, msg: HeartbeatMessage) {
        // Delegate to StatusService if available (Option A: Composition Root injection)
        if let Some(status_service) = &self.status_service {
            status_service.handle_heartbeat(msg).await;
        } else {
            // Safety fallback: if StatusService is missing
            tracing::error!("StatusService is not initialized! Heartbeat ignored.");
        }
    }


}
