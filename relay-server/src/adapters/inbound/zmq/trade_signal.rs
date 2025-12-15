//! Trade signal handler
//!
//! Handles incoming trade signals from Master EAs, applies filters,
//! transforms signals, and distributes them to Slave EAs.

use super::MessageHandler;
use crate::domain::models::{MasterSettings, SymbolConverter, TradeGroupMember, TradeSignal};

impl MessageHandler {
    /// Handle trade signals and process copying
    pub(super) async fn handle_trade_signal(&self, signal: TradeSignal) {
        tracing::info!("Processing trade signal: {:?}", signal);

        // Notify WebSocket clients
        let _ = self.broadcast_tx.send(format!(
            "trade_received:{}:{}:{}",
            signal.source_account,
            signal.symbol.as_deref().unwrap_or("?"),
            signal.lots.unwrap_or(0.0)
        ));

        // Get master settings for symbol prefix/suffix
        let master_settings = match self.db.get_trade_group(&signal.source_account).await {
            Ok(Some(tg)) => tg.master_settings,
            Ok(None) => {
                tracing::warn!(
                    "TradeGroup not found for master {}, using defaults",
                    signal.source_account
                );
                MasterSettings::default()
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get TradeGroup for master {}: {}",
                    signal.source_account,
                    e
                );
                return;
            }
        };

        // Get all members (slaves) for this master account
        let members = match self.db.get_members(&signal.source_account).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!(
                    "Failed to get members for master {}: {}",
                    signal.source_account,
                    e
                );
                return;
            }
        };

        for member in &members {
            // Apply filters
            if !self.copy_engine.should_copy_trade(&signal, member) {
                tracing::debug!(
                    "Trade filtered out for slave account: {}",
                    member.slave_account
                );
                continue;
            }

            // Process the trade copy
            self.process_trade_copy(&signal, member, &master_settings)
                .await;
        }
    }

    /// Process a single trade copy for a specific member
    async fn process_trade_copy(
        &self,
        signal: &TradeSignal,
        member: &TradeGroupMember,
        master_settings: &MasterSettings,
    ) {
        // Transform signal
        // SymbolConverter removes master's prefix/suffix and applies slave's prefix/suffix + mappings
        let converter = SymbolConverter::from_settings(master_settings, &member.slave_settings);

        match self
            .copy_engine
            .transform_signal(signal.clone(), member, &converter)
        {
            Ok(transformed) => {
                tracing::info!(
                    "Copying trade to {}: {} {} lots",
                    member.slave_account,
                    transformed.symbol.as_deref().unwrap_or("?"),
                    transformed.lots.unwrap_or(0.0)
                );

                // Send to specific Master-Slave pair using trade/{master}/{slave} topic
                // Each slave subscribes to their specific topic for precise filtering
                if let Err(e) = self
                    .publisher
                    .send_trade_signal(&signal.source_account, &member.slave_account, &transformed)
                    .await
                {
                    tracing::error!("Failed to send signal to trade group: {}", e);
                } else {
                    tracing::debug!(
                        "Sent signal on topic 'trade/{}/{}' for slave '{}'",
                        signal.source_account,
                        member.slave_account,
                        member.slave_account
                    );

                    // Notify WebSocket clients
                    let _ = self.broadcast_tx.send(format!(
                        "trade_copied:{}:{}:{}:{}",
                        member.slave_account,
                        transformed.symbol.as_deref().unwrap_or("?"),
                        transformed.lots.unwrap_or(0.0),
                        member.id
                    ));
                }
            }
            Err(e) => {
                tracing::error!("Failed to transform signal: {}", e);
            }
        }
    }
}
