//! Trade signal handler
//!
//! Handles incoming trade signals from Master EAs, applies filters,
//! transforms signals, and distributes them to Slave EAs.

use super::MessageHandler;
use crate::models::{CopySettings, SymbolConverter, TradeSignal};

impl MessageHandler {
    /// Handle trade signals and process copying
    pub(super) async fn handle_trade_signal(&self, signal: TradeSignal) {
        tracing::info!("Processing trade signal: {:?}", signal);

        // Notify WebSocket clients
        let _ = self.broadcast_tx.send(format!(
            "trade_received:{}:{}:{}",
            signal.source_account, signal.symbol, signal.lots
        ));

        // Get copy settings for this master account from database
        let settings = match self
            .db
            .get_copy_settings_for_master(&signal.source_account)
            .await
        {
            Ok(settings) => settings,
            Err(e) => {
                tracing::error!(
                    "Failed to get copy settings for master {}: {}",
                    signal.source_account,
                    e
                );
                return;
            }
        };

        for setting in &settings {
            // Apply filters
            if !self.copy_engine.should_copy_trade(&signal, setting) {
                tracing::debug!(
                    "Trade filtered out for slave account: {}",
                    setting.slave_account
                );
                continue;
            }

            // Process the trade copy
            self.process_trade_copy(&signal, setting).await;
        }
    }

    /// Process a single trade copy for a specific setting
    async fn process_trade_copy(&self, signal: &TradeSignal, setting: &CopySettings) {
        // Transform signal
        let converter = SymbolConverter {
            prefix_remove: None,
            suffix_remove: None,
            prefix_add: None,
            suffix_add: None,
        };

        match self
            .copy_engine
            .transform_signal(signal.clone(), setting, &converter)
        {
            Ok(transformed) => {
                tracing::info!(
                    "Copying trade to {}: {} {} lots",
                    setting.slave_account,
                    transformed.symbol,
                    transformed.lots
                );

                // Send to trade group using PUB/SUB with master_account as topic
                // This allows multiple slaves to subscribe to the same master's trades
                if let Err(e) = self
                    .zmq_sender
                    .send_signal(&setting.master_account, &transformed)
                    .await
                {
                    tracing::error!("Failed to send signal to trade group: {}", e);
                } else {
                    tracing::debug!(
                        "Sent signal to trade group '{}' for slave '{}'",
                        setting.master_account,
                        setting.slave_account
                    );

                    // Notify WebSocket clients
                    let _ = self.broadcast_tx.send(format!(
                        "trade_copied:{}:{}:{}:{}",
                        setting.slave_account, transformed.symbol, transformed.lots, setting.id
                    ));
                }
            }
            Err(e) => {
                tracing::error!("Failed to transform signal: {}", e);
            }
        }
    }
}
