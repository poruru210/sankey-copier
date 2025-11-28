// relay-server/src/message_handler/position_snapshot.rs
//
// Handler for PositionSnapshot messages from Master EAs.
// Routes position snapshots to all connected Slave EAs for synchronization.

use super::MessageHandler;
use crate::models::PositionSnapshotMessage;

impl MessageHandler {
    /// Handle PositionSnapshot message from Master EA
    ///
    /// When a Master EA sends its current positions (e.g., after restart),
    /// this handler routes the snapshot to all connected Slave EAs.
    pub(super) async fn handle_position_snapshot(&self, snapshot: PositionSnapshotMessage) {
        tracing::info!(
            "Processing PositionSnapshot from {}: {} positions",
            snapshot.source_account,
            snapshot.positions.len()
        );

        // Notify WebSocket clients
        let _ = self.broadcast_tx.send(format!(
            "position_snapshot:{}:{}",
            snapshot.source_account,
            snapshot.positions.len()
        ));

        // Get all members (slaves) for this master account
        let members = match self.db.get_members(&snapshot.source_account).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!(
                    "Failed to get members for master {}: {}",
                    snapshot.source_account,
                    e
                );
                return;
            }
        };

        if members.is_empty() {
            tracing::debug!(
                "No slaves connected to master {}, skipping snapshot distribution",
                snapshot.source_account
            );
            return;
        }

        // Route snapshot to each connected slave via config publisher
        for member in &members {
            if let Err(e) = self
                .config_sender
                .publish_to_topic(&member.slave_account, &snapshot)
                .await
            {
                tracing::error!(
                    "Failed to send PositionSnapshot to slave {}: {}",
                    member.slave_account,
                    e
                );
            } else {
                tracing::debug!(
                    "Sent PositionSnapshot to slave {} ({} positions)",
                    member.slave_account,
                    snapshot.positions.len()
                );
            }
        }

        tracing::info!(
            "Distributed PositionSnapshot from {} to {} slaves",
            snapshot.source_account,
            members.len()
        );
    }
}
