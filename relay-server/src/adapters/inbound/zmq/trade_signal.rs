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
        // Retrieve detected symbols for the slave
        let detected_symbols = self
            .connection_manager
            .get_slave(&member.slave_account)
            .await
            .and_then(|conn| conn.detected_symbols);

        // Transform signal
        // SymbolConverter removes master's prefix/suffix and applies slave's prefix/suffix + mappings
        let converter = SymbolConverter::from_settings(master_settings, &member.slave_settings)
            .with_auto_mapping(
                self.config.symbol_mapping.synonym_groups.clone(),
                detected_symbols,
            );

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

#[cfg(test)]
mod tests {
    // use super::*;
    use crate::adapters::inbound::zmq::test_helpers::{
        create_test_context, create_test_context_with_config, create_test_trade_signal,
    };
    use crate::domain::models::{LotCalculationMode, SlaveSettings};

    #[tokio::test]
    async fn test_handle_trade_signal_with_matching_setting() {
        let ctx = create_test_context().await;
        let signal = create_test_trade_signal();

        // Create TradeGroup for master and add member to database
        ctx.db.create_trade_group("MASTER_001").await.unwrap();

        let slave_settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            config_version: 1,
            lot_multiplier: Some(1.0),
            ..SlaveSettings::default()
        };
        ctx.db
            .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
            .await
            .unwrap();

        // Process trade signal (should not panic)
        ctx.handle_trade_signal(signal).await;

        ctx.cleanup().await;
    }

    #[tokio::test]
    async fn test_handle_trade_signal_no_matching_master() {
        let ctx = create_test_context().await;
        let mut signal = create_test_trade_signal();
        signal.source_account = "OTHER_MASTER".to_string();

        ctx.db.create_trade_group("MASTER_001").await.unwrap();

        let slave_settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            config_version: 1,
            lot_multiplier: Some(1.0),
            ..SlaveSettings::default()
        };
        ctx.db
            .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
            .await
            .unwrap();

        // Process trade signal (should be filtered out, no panic)
        ctx.handle_trade_signal(signal).await;

        ctx.cleanup().await;
    }

    #[tokio::test]
    async fn test_handle_trade_signal_disabled_setting() {
        let ctx = create_test_context().await;
        let signal = create_test_trade_signal();

        ctx.db.create_trade_group("MASTER_001").await.unwrap();

        let slave_settings = SlaveSettings {
            lot_calculation_mode: LotCalculationMode::default(),
            config_version: 1,
            lot_multiplier: Some(1.0),
            ..SlaveSettings::default()
        };
        // Add member and then disable it
        ctx.db
            .add_member("MASTER_001", "SLAVE_001", slave_settings, 0)
            .await
            .unwrap();
        ctx.db
            .update_member_runtime_status("MASTER_001", "SLAVE_001", 0)
            .await
            .unwrap(); // STATUS_DISABLED = 0

        // Process trade signal (should be filtered out, no panic)
        ctx.handle_trade_signal(signal).await;

        ctx.cleanup().await;
    }

    #[tokio::test]
    async fn test_handle_trade_signal_with_auto_mapping() {
        // 1. Setup Config with Synonym Groups
        let mut config = crate::config::Config::default();
        config.symbol_mapping.synonym_groups = vec![vec!["XAUUSD".to_string(), "GOLD".to_string()]];

        let ctx = create_test_context_with_config(config).await;

        // 2. Setup Master and Slave
        ctx.db.create_trade_group("MASTER_GOLD").await.unwrap();

        let slave_settings = SlaveSettings {
            lot_multiplier: Some(1.0),
            ..Default::default()
        };
        ctx.db
            .add_member("MASTER_GOLD", "SLAVE_GOLD", slave_settings, 2)
            .await
            .unwrap();

        // 3. Register Slave with detected symbols (Simulate "GOLD" detected)
        let register_msg = crate::domain::models::RegisterMessage {
            message_type: "Register".to_string(),
            account_id: "SLAVE_GOLD".to_string(),
            ea_type: "Slave".to_string(),
            platform: "MT5".to_string(),
            account_number: 12345,
            broker: "Test Broker".to_string(),
            account_name: "Test Account".to_string(),
            server: "Test-Server".to_string(),
            currency: "USD".to_string(),
            leverage: 100,
            timestamp: chrono::Utc::now().to_rfc3339(),
            detected_symbols: Some(vec!["GOLD".to_string()]),
        };
        ctx.connection_manager.register_ea(&register_msg).await;

        // 4. Send Trade Signal for "XAUUSD" (which should map to "GOLD")
        let mut signal = create_test_trade_signal();
        signal.source_account = "MASTER_GOLD".to_string();
        signal.symbol = Some("XAUUSD".to_string());

        // 5. Verify signal was published with "GOLD"
        let mut rx = ctx._broadcast_rx.resubscribe();

        // Process signal
        ctx.handle_trade_signal(signal).await;

        // Consume messages until we find "trade_copied"
        // Might get "trade_received" first.
        loop {
            // Add timeout to prevent infinite loop
            let msg = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx.recv())
                .await
                .expect("Timed out waiting for trade_copied")
                .unwrap();

            if msg.starts_with("trade_copied") {
                // Check format: trade_copied:SLAVE_GOLD:GOLD:0.1:ID
                assert!(msg.contains("SLAVE_GOLD"));
                assert!(
                    msg.contains(":GOLD:"),
                    "Should contain mapped symbol GOLD, got: {}",
                    msg
                );
                break;
            }
        }

        ctx.cleanup().await;
    }
}
