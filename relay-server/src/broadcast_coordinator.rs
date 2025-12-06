//! Broadcast Coordinator
//!
//! Centralizes WebSocket broadcast logic for settings updates with change detection.
//! Manages warning_codes cache and ensures broadcasts only happen when data changes.

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

use crate::models::{SlaveConfigWithMaster, WarningCode};

/// Coordinates WebSocket broadcasts with change detection and caching
///
/// This service owns the warning_codes cache and provides a single point
/// for broadcast decisions, eliminating duplicate logic across handlers.
#[derive(Clone)]
pub struct BroadcastCoordinator {
    /// Cache of last broadcast warning_codes per slave account
    cache: Arc<RwLock<HashMap<String, Vec<WarningCode>>>>,
    /// WebSocket broadcast channel
    broadcast_tx: broadcast::Sender<String>,
}

impl BroadcastCoordinator {
    /// Create a new BroadcastCoordinator
    pub fn new(broadcast_tx: broadcast::Sender<String>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Broadcast settings update if warning_codes or status changed
    ///
    /// # Arguments
    /// * `slave_account` - The slave account identifier
    /// * `new_warning_codes` - Current warning codes from Status Engine
    /// * `payload` - Complete settings payload to broadcast
    /// * `force` - If true, broadcast even if warning_codes unchanged (e.g., status changed)
    ///
    /// # Returns
    /// `true` if broadcast was sent, `false` if skipped (no changes)
    pub async fn broadcast_settings_if_changed(
        &self,
        slave_account: &str,
        new_warning_codes: Vec<WarningCode>,
        payload: SlaveConfigWithMaster,
        force: bool,
    ) -> bool {
        // Check if warning_codes changed
        let cache = self.cache.read().await;
        let cached_entry = cache.get(slave_account).cloned();
        drop(cache);

        let warning_codes_changed = match &cached_entry {
            Some(previous) => new_warning_codes != *previous,
            None => true, // First broadcast for this account - always send
        };

        tracing::info!(
            slave = %slave_account,
            cached_warnings = ?cached_entry,
            new_warnings = ?new_warning_codes,
            warning_codes_changed = warning_codes_changed,
            force = force,
            will_broadcast = force || warning_codes_changed,
            "[BroadcastCoordinator] Decision check"
        );

        // Broadcast if forced or warning_codes changed
        if force || warning_codes_changed {
            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(slave_account.to_string(), new_warning_codes.clone());
            drop(cache);

            // Broadcast via WebSocket
            if let Ok(json) = serde_json::to_string(&payload) {
                let _ = self.broadcast_tx.send(format!("settings_updated:{}", json));
                tracing::debug!(
                    slave = %slave_account,
                    master = %payload.master_account,
                    status = payload.status,
                    warning_codes_changed = warning_codes_changed,
                    forced = force,
                    "Broadcast settings_updated via BroadcastCoordinator"
                );
                return true;
            }
        }

        false
    }

    /// Clear cache entry for a slave (e.g., when slave is removed)
    #[allow(dead_code)]
    pub async fn clear_cache(&self, slave_account: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(slave_account);
    }

    /// Get cached warning_codes for a slave (for testing/debugging)
    #[allow(dead_code)]
    pub async fn get_cached_warnings(&self, slave_account: &str) -> Option<Vec<WarningCode>> {
        let cache = self.cache.read().await;
        cache.get(slave_account).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SlaveSettings;

    fn create_test_payload(
        master: &str,
        slave: &str,
        status: i32,
        warning_codes: Vec<WarningCode>,
    ) -> SlaveConfigWithMaster {
        SlaveConfigWithMaster {
            master_account: master.to_string(),
            slave_account: slave.to_string(),
            status,
            enabled_flag: true,
            warning_codes,
            slave_settings: SlaveSettings::default(),
        }
    }

    #[tokio::test]
    async fn broadcasts_on_first_call() {
        let (tx, mut rx) = broadcast::channel(16);
        let coordinator = BroadcastCoordinator::new(tx);

        let payload = create_test_payload("MASTER1", "SLAVE1", 2, vec![]);

        let broadcast = coordinator
            .broadcast_settings_if_changed("SLAVE1", vec![], payload, false)
            .await;

        assert!(broadcast, "Should broadcast on first call (cache empty)");
        let msg = rx.try_recv().unwrap();
        assert!(msg.starts_with("settings_updated:"));
    }

    #[tokio::test]
    async fn skips_broadcast_when_unchanged() {
        let (tx, mut rx) = broadcast::channel(16);
        let coordinator = BroadcastCoordinator::new(tx);

        let warnings = vec![WarningCode::SlaveOffline];
        let payload1 = create_test_payload("MASTER1", "SLAVE1", 1, warnings.clone());
        let payload2 = create_test_payload("MASTER1", "SLAVE1", 1, warnings.clone());

        // First call - should broadcast
        coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings.clone(), payload1, false)
            .await;
        rx.try_recv().unwrap(); // Consume first message

        // Second call with same warning_codes - should skip
        let broadcast = coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings, payload2, false)
            .await;

        assert!(!broadcast, "Should skip when warning_codes unchanged");
        assert!(rx.try_recv().is_err(), "No message should be sent");
    }

    #[tokio::test]
    async fn broadcasts_when_warning_codes_change() {
        let (tx, mut rx) = broadcast::channel(16);
        let coordinator = BroadcastCoordinator::new(tx);

        let warnings1 = vec![WarningCode::SlaveOffline];
        let warnings2 = vec![WarningCode::SlaveAutoTradingDisabled];

        let payload1 = create_test_payload("MASTER1", "SLAVE1", 1, warnings1.clone());
        let payload2 = create_test_payload("MASTER1", "SLAVE1", 1, warnings2.clone());

        // First call
        coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings1, payload1, false)
            .await;
        rx.try_recv().unwrap();

        // Second call with different warning_codes
        let broadcast = coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings2, payload2, false)
            .await;

        assert!(broadcast, "Should broadcast when warning_codes change");
        let msg = rx.try_recv().unwrap();
        assert!(msg.contains("slave_auto_trading_disabled"));
    }

    #[tokio::test]
    async fn force_flag_overrides_change_detection() {
        let (tx, mut rx) = broadcast::channel(16);
        let coordinator = BroadcastCoordinator::new(tx);

        let warnings = vec![WarningCode::SlaveOffline];
        let payload1 = create_test_payload("MASTER1", "SLAVE1", 1, warnings.clone());
        let payload2 = create_test_payload("MASTER1", "SLAVE1", 2, warnings.clone());

        // First call
        coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings.clone(), payload1, false)
            .await;
        rx.try_recv().unwrap();

        // Second call with force=true (e.g., status changed)
        let broadcast = coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings, payload2, true)
            .await;

        assert!(broadcast, "Should broadcast when forced");
        let msg = rx.try_recv().unwrap();
        assert!(msg.contains("\"status\":2"));
    }

    #[tokio::test]
    async fn clear_cache_removes_entry() {
        let (tx, _rx) = broadcast::channel(16);
        let coordinator = BroadcastCoordinator::new(tx);

        let warnings = vec![WarningCode::SlaveOffline];
        let payload = create_test_payload("MASTER1", "SLAVE1", 1, warnings.clone());

        coordinator
            .broadcast_settings_if_changed("SLAVE1", warnings, payload, false)
            .await;

        assert!(coordinator.get_cached_warnings("SLAVE1").await.is_some());

        coordinator.clear_cache("SLAVE1").await;

        assert!(coordinator.get_cached_warnings("SLAVE1").await.is_none());
    }
}
