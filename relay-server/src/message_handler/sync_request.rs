// relay-server/src/message_handler/sync_request.rs
//
// Handler for SyncRequest messages from Slave EAs.
// Routes sync requests to the specified Master EA.

use super::MessageHandler;
use crate::models::SyncRequestMessage;

impl MessageHandler {
    /// Handle SyncRequest message from Slave EA
    ///
    /// When a Slave EA starts up and needs to sync with its Master,
    /// it sends a SyncRequest. This handler routes the request to the Master EA.
    pub(super) async fn handle_sync_request(&self, request: SyncRequestMessage) {
        tracing::info!(
            "Processing SyncRequest from {} for master {}",
            request.slave_account,
            request.master_account
        );

        // Notify WebSocket clients
        let _ = self.broadcast_tx.send(format!(
            "sync_request:{}:{}",
            request.slave_account, request.master_account
        ));

        // Verify the slave is actually a member of this master's trade group
        let members = match self.db.get_members(&request.master_account).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!(
                    "Failed to get members for master {}: {}",
                    request.master_account,
                    e
                );
                return;
            }
        };

        let is_valid_member = members
            .iter()
            .any(|m| m.slave_account == request.slave_account);

        if !is_valid_member {
            tracing::warn!(
                "SyncRequest from {} rejected: not a member of master {}",
                request.slave_account,
                request.master_account
            );
            return;
        }

        // Route sync request to Master EA via config publisher
        let topic = format!("config/{}", request.master_account);
        if let Err(e) = self.publisher.publish_to_topic(&topic, &request).await {
            tracing::error!(
                "Failed to send SyncRequest to master {}: {}",
                request.master_account,
                e
            );
        } else {
            tracing::debug!(
                "Sent SyncRequest from {} to master {}",
                request.slave_account,
                request.master_account
            );
        }
    }
}
