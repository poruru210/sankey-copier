use std::sync::{atomic::AtomicU64, atomic::Ordering, Arc};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    config_builder::{ConfigBuilder, SlaveConfigBundle, SlaveConfigContext},
    domain::services::status_calculator::{
        evaluate_master_status, evaluate_member_status, ConnectionSnapshot, MasterClusterSnapshot,
        MasterIntent, MasterStatusResult, MemberStatusResult, SlaveIntent, SlaveRuntimeTarget,
    },
    ports::outbound::{ConnectionManager, TradeGroupRepository},
};

use crate::domain::models::WarningCode;

#[allow(clippy::too_many_arguments)]
pub fn log_slave_runtime_trace(
    source: &'static str,
    master_account: &str,
    slave_account: &str,
    previous_status: i32,
    new_status: i32,
    allow_new_orders: bool,
    warning_codes: &[WarningCode],
    cluster_size: usize,
    masters_all_connected: bool,
) {
    tracing::event!(
        target: "status_engine",
        tracing::Level::INFO,
        source,
        master = %master_account,
        slave = %slave_account,
        previous_status = previous_status,
        status = new_status,
        status_changed = previous_status != new_status,
        allow_new_orders = allow_new_orders,
        warning_count = warning_codes.len(),
        cluster_size = cluster_size,
        masters_all_connected = masters_all_connected,
        warnings = ?warning_codes,
        "slave runtime evaluation"
    );
}

/// Helper that centralizes runtime snapshot gathering for Master/Slave pairs.
#[derive(Clone)]
pub struct RuntimeStatusUpdater {
    db: Arc<dyn TradeGroupRepository>,
    connection_manager: Arc<dyn ConnectionManager>,
    metrics: Arc<RuntimeStatusMetrics>,
}

impl RuntimeStatusUpdater {
    pub fn with_metrics(
        db: Arc<dyn TradeGroupRepository>,
        connection_manager: Arc<dyn ConnectionManager>,
        metrics: Arc<RuntimeStatusMetrics>,
    ) -> Self {
        Self {
            db,
            connection_manager,
            metrics,
        }
    }

    #[instrument(skip(self), fields(slave_account = %slave_account))]
    pub async fn slave_connection_snapshot(&self, slave_account: &str) -> ConnectionSnapshot {
        let slave_conn = self.connection_manager.get_slave(slave_account).await;
        let snapshot = ConnectionSnapshot {
            connection_status: slave_conn.as_ref().map(|conn| conn.status),
            is_trade_allowed: slave_conn
                .as_ref()
                .map(|conn| conn.is_trade_allowed)
                .unwrap_or(false),
        };
        tracing::debug!(
            target: "status",
            slave_account = %slave_account,
            connection_status = ?snapshot.connection_status,
            is_trade_allowed = snapshot.is_trade_allowed,
            "slave_connection_snapshot"
        );
        snapshot
    }

    #[instrument(skip(self), fields(master_account = %master_account))]
    pub async fn evaluate_master_runtime_status(
        &self,
        master_account: &str,
    ) -> Option<MasterStatusResult> {
        let trade_group = match self.db.get_trade_group(master_account).await {
            Ok(Some(tg)) => tg,
            Ok(None) => {
                tracing::debug!(
                    master_account = %master_account,
                    "TradeGroup missing while evaluating master runtime status"
                );
                self.metrics.record_master_eval_failure();
                return None;
            }
            Err(err) => {
                tracing::error!(
                    master_account = %master_account,
                    error = %err,
                    "Failed to load TradeGroup while evaluating master runtime status"
                );
                self.metrics.record_master_eval_failure();
                return None;
            }
        };

        let master_conn = self.connection_manager.get_master(master_account).await;
        let snapshot = ConnectionSnapshot {
            connection_status: master_conn.as_ref().map(|conn| conn.status),
            is_trade_allowed: master_conn
                .as_ref()
                .map(|conn| conn.is_trade_allowed)
                .unwrap_or(false),
        };

        let result = evaluate_master_status(
            MasterIntent {
                web_ui_enabled: trade_group.master_settings.enabled,
            },
            snapshot,
        );

        tracing::info!(
            master_account = %master_account,
            status = result.status,
            warning_codes = ?result.warning_codes,
            connection_status = ?snapshot.connection_status,
            is_trade_allowed = snapshot.is_trade_allowed,
            web_ui_enabled = trade_group.master_settings.enabled,
            "[RuntimeStatusUpdater] Evaluated master runtime status"
        );
        self.metrics.record_master_eval_success();
        Some(result)
    }

    #[instrument(skip(self, target), fields(slave_account = %target.slave_account, master_account = %target.master_account))]
    pub async fn build_slave_bundle(&self, target: SlaveRuntimeTarget<'_>) -> SlaveConfigBundle {
        // Get the specific Master's status (not the entire cluster)
        let master_result = self
            .evaluate_master_runtime_status(target.master_account)
            .await
            .unwrap_or_default();

        let slave_snapshot = self.slave_connection_snapshot(target.slave_account).await;
        let master_equity = self
            .connection_manager
            .get_master(target.master_account)
            .await
            .map(|conn| conn.equity);

        let bundle = ConfigBuilder::build_slave_config(SlaveConfigContext {
            slave_account: target.slave_account.to_string(),
            master_account: target.master_account.to_string(),
            trade_group_id: target.trade_group_id.to_string(),
            intent: SlaveIntent {
                web_ui_enabled: target.enabled_flag,
            },
            slave_connection_snapshot: slave_snapshot,
            master_status_result: master_result.clone(),
            slave_settings: target.slave_settings,
            master_equity,
            timestamp: Utc::now(),
        });

        tracing::debug!(
            target: "status",
            slave_account = %target.slave_account,
            master_account = %target.master_account,
            status = bundle.status_result.status,
            allow_new_orders = bundle.status_result.allow_new_orders,
            warning_count = bundle.status_result.warning_codes.len(),
            master_status = master_result.status,
            "built slave config bundle (per-connection)"
        );
        self.metrics.record_slave_bundle(1);

        bundle
    }

    /// Evaluate the runtime status of a specific Member (Master-Slave connection).
    /// Unlike the old cluster-based evaluation, this evaluates based on the specific Master only.
    #[instrument(skip(self, target), fields(slave_account = %target.slave_account, master_account = %target.master_account))]
    pub async fn evaluate_member_runtime_status(
        &self,
        target: SlaveRuntimeTarget<'_>,
    ) -> MemberStatusResult {
        let slave_snapshot = self.slave_connection_snapshot(target.slave_account).await;
        self.evaluate_member_runtime_status_with_snapshot(target, slave_snapshot)
            .await
    }

    /// Helper to evaluate status with an explicit snapshot (useful for "Old" state evaluation)
    pub async fn evaluate_member_runtime_status_with_snapshot(
        &self,
        target: SlaveRuntimeTarget<'_>,
        slave_snapshot: ConnectionSnapshot,
    ) -> MemberStatusResult {
        // Get the specific Master's status
        let master_result = self
            .evaluate_master_runtime_status(target.master_account)
            .await
            .unwrap_or_default();

        let result = evaluate_member_status(
            SlaveIntent {
                web_ui_enabled: target.enabled_flag,
            },
            slave_snapshot,
            &master_result,
        );

        tracing::debug!(
            target: "status",
            slave_account = %target.slave_account,
            master_account = %target.master_account,
            status = result.status,
            allow_new_orders = result.allow_new_orders,
            warning_count = result.warning_codes.len(),
            master_status = master_result.status,
            "evaluated member runtime status (per-connection)"
        );
        self.metrics.record_slave_eval_success();

        result
    }

    /// Build a cluster snapshot for all Masters connected to a Slave.
    /// This is kept for account-level aggregation (e.g., Web UI Slave node badge).
    #[allow(dead_code)]
    #[instrument(skip(self), fields(slave_account = %slave_account))]
    pub async fn master_cluster_snapshot(&self, slave_account: &str) -> MasterClusterSnapshot {
        match self.db.get_masters_for_slave(slave_account).await {
            Ok(master_accounts) => {
                let mut results = Vec::with_capacity(master_accounts.len());
                for master in master_accounts {
                    let result = self
                        .evaluate_master_runtime_status(&master)
                        .await
                        .unwrap_or_default();
                    results.push(result);
                }
                MasterClusterSnapshot::with_status_results(results)
            }
            Err(err) => {
                tracing::error!(
                    slave_account = %slave_account,
                    error = %err,
                    "Failed to build master cluster snapshot"
                );
                self.metrics.record_slave_eval_failure();
                MasterClusterSnapshot::default()
            }
        }
    }
}

#[derive(Default)]
pub struct RuntimeStatusMetrics {
    master_evaluations_total: AtomicU64,
    master_evaluations_failed: AtomicU64,
    slave_evaluations_total: AtomicU64,
    slave_evaluations_failed: AtomicU64,
    slave_bundles_built: AtomicU64,
    last_cluster_size: AtomicU64,
}

impl RuntimeStatusMetrics {
    pub fn record_master_eval_success(&self) {
        self.master_evaluations_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_master_eval_failure(&self) {
        self.master_evaluations_total
            .fetch_add(1, Ordering::Relaxed);
        self.master_evaluations_failed
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_slave_eval_success(&self) {
        self.slave_evaluations_total.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_slave_eval_failure(&self) {
        self.slave_evaluations_total.fetch_add(1, Ordering::Relaxed);
        self.slave_evaluations_failed
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_slave_bundle(&self, cluster_size: u64) {
        self.slave_bundles_built.fetch_add(1, Ordering::Relaxed);
        self.last_cluster_size
            .store(cluster_size, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> RuntimeStatusMetricsSnapshot {
        RuntimeStatusMetricsSnapshot {
            master_evaluations_total: self.master_evaluations_total.load(Ordering::Relaxed),
            master_evaluations_failed: self.master_evaluations_failed.load(Ordering::Relaxed),
            slave_evaluations_total: self.slave_evaluations_total.load(Ordering::Relaxed),
            slave_evaluations_failed: self.slave_evaluations_failed.load(Ordering::Relaxed),
            slave_bundles_built: self.slave_bundles_built.load(Ordering::Relaxed),
            last_cluster_size: self.last_cluster_size.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RuntimeStatusMetricsSnapshot {
    pub master_evaluations_total: u64,
    pub master_evaluations_failed: u64,
    pub slave_evaluations_total: u64,
    pub slave_evaluations_failed: u64,
    pub slave_bundles_built: u64,
    pub last_cluster_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{
        ConnectionStatus, EaConnection, EaType, SlaveSettings, TradeGroup, TradeGroupMember,
    };
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;

    // --- Mocks Definitions ---

    mock! {
        pub TradeGroupRepository {}
        #[async_trait]
        impl TradeGroupRepository for TradeGroupRepository {
            async fn get_trade_group(&self, id: &str) -> anyhow::Result<Option<TradeGroup>>;
            async fn create_trade_group(&self, id: &str) -> anyhow::Result<TradeGroup>;
            async fn get_members(&self, master_id: &str) -> anyhow::Result<Vec<TradeGroupMember>>;
            async fn get_settings_for_slave(&self, slave_id: &str) -> anyhow::Result<Vec<crate::domain::models::SlaveConfigWithMaster>>;
            async fn update_member_runtime_status(&self, master_id: &str, slave_id: &str, status: i32) -> anyhow::Result<()>;
            async fn get_masters_for_slave(&self, slave_account: &str) -> anyhow::Result<Vec<String>>;
        }
    }

    mock! {
        pub ConnectionManager {}
        #[async_trait]
        impl ConnectionManager for ConnectionManager {
            async fn get_master(&self, account_id: &str) -> Option<EaConnection>;
            async fn get_slave(&self, account_id: &str) -> Option<EaConnection>;

            async fn update_heartbeat(&self, msg: crate::domain::models::HeartbeatMessage) -> bool;
        }
    }

    // --- Helpers ---

    fn create_updater(
        repo: MockTradeGroupRepository,
        conn: MockConnectionManager,
    ) -> RuntimeStatusUpdater {
        RuntimeStatusUpdater::with_metrics(
            Arc::new(repo),
            Arc::new(conn),
            Arc::new(RuntimeStatusMetrics::default()),
        )
    }

    fn default_trade_group(id: &str, enabled: bool) -> TradeGroup {
        let mut tg = TradeGroup::new(id.to_string());
        tg.master_settings.enabled = enabled;
        tg
    }

    fn online_connection(id: &str, ea_type: EaType, trade_allowed: bool) -> EaConnection {
        EaConnection {
            account_id: id.to_string(),
            ea_type,
            status: ConnectionStatus::Online,
            is_trade_allowed: trade_allowed,
            ..Default::default()
        }
    }

    // --- Tests for evaluate_master_runtime_status ---

    #[tokio::test]
    async fn evaluate_master_missing_trade_group_returns_none() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mock_conn = MockConnectionManager::new();

        mock_repo
            .expect_get_trade_group()
            .with(eq("MASTER_1"))
            .returning(|_| Ok(None)); // Missing

        let updater = create_updater(mock_repo, mock_conn);
        let result = updater.evaluate_master_runtime_status("MASTER_1").await;

        assert!(result.is_none());

        let snapshot = updater.metrics.snapshot();
        assert_eq!(snapshot.master_evaluations_failed, 1);
    }

    #[tokio::test]
    async fn evaluate_master_db_error_returns_none() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mock_conn = MockConnectionManager::new();

        mock_repo
            .expect_get_trade_group()
            .with(eq("MASTER_1"))
            .returning(|_| Err(anyhow::anyhow!("DB Error"))); // Error

        let updater = create_updater(mock_repo, mock_conn);
        let result = updater.evaluate_master_runtime_status("MASTER_1").await;

        assert!(result.is_none());

        let snapshot = updater.metrics.snapshot();
        assert_eq!(snapshot.master_evaluations_failed, 1);
    }

    #[tokio::test]
    async fn evaluate_master_disconnected() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_conn = MockConnectionManager::new();

        mock_repo
            .expect_get_trade_group()
            .with(eq("MASTER_1"))
            .returning(|_| Ok(Some(default_trade_group("MASTER_1", true))));

        mock_conn
            .expect_get_master()
            .with(eq("MASTER_1"))
            .returning(|_| None); // Disconnected

        let updater = create_updater(mock_repo, mock_conn);
        let result = updater
            .evaluate_master_runtime_status("MASTER_1")
            .await
            .unwrap();

        assert_ne!(result.status, 2); // Not ENABLED
    }

    #[tokio::test]
    async fn evaluate_master_enabled_and_trade_allowed() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_conn = MockConnectionManager::new();

        mock_repo
            .expect_get_trade_group()
            .with(eq("MASTER_1"))
            .returning(|_| Ok(Some(default_trade_group("MASTER_1", true))));

        mock_conn
            .expect_get_master()
            .with(eq("MASTER_1"))
            .returning(|_| Some(online_connection("MASTER_1", EaType::Master, true)));

        let updater = create_updater(mock_repo, mock_conn);
        let result = updater
            .evaluate_master_runtime_status("MASTER_1")
            .await
            .unwrap();

        // ENABLED
        assert_eq!(result.status, 2);
        assert!(result.warning_codes.is_empty());

        let snapshot = updater.metrics.snapshot();
        assert_eq!(snapshot.master_evaluations_total, 1);
    }

    // --- Tests for evaluate_member_runtime_status ---

    #[tokio::test]
    async fn evaluate_member_propagates_master_status() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_conn = MockConnectionManager::new();

        // Setup Master: Enabled but Trade Disabled (Warning)
        mock_repo
            .expect_get_trade_group()
            .with(eq("MASTER_1"))
            .returning(|_| Ok(Some(default_trade_group("MASTER_1", true))));

        mock_conn
            .expect_get_master()
            .with(eq("MASTER_1"))
            .returning(|_| Some(online_connection("MASTER_1", EaType::Master, false)));

        // Setup Slave: Online
        mock_conn
            .expect_get_slave()
            .with(eq("SLAVE_1"))
            .returning(|_| Some(online_connection("SLAVE_1", EaType::Slave, true)));

        let updater = create_updater(mock_repo, mock_conn);

        let target = SlaveRuntimeTarget {
            master_account: "MASTER_1",
            trade_group_id: "MASTER_1",
            slave_account: "SLAVE_1",
            enabled_flag: true,
            slave_settings: &SlaveSettings::default(),
        };

        let result = updater.evaluate_member_runtime_status(target).await;

        // Master trade not allowed -> Master Status DISABLED(0) -> Member Status ENABLED(1) (Waiting)
        assert_eq!(result.status, 1);
        // allow_new_orders depends ONLY on Slave state, so it should be TRUE
        assert!(result.allow_new_orders);
    }

    // --- Tests for build_slave_bundle ---

    #[tokio::test]
    async fn build_slave_bundle_assembles_correctly() {
        let mut mock_repo = MockTradeGroupRepository::new();
        let mut mock_conn = MockConnectionManager::new();

        // Master OK
        mock_repo
            .expect_get_trade_group()
            .returning(|_| Ok(Some(default_trade_group("MASTER_1", true))));
        mock_conn
            .expect_get_master()
            .returning(|_| Some(online_connection("MASTER_1", EaType::Master, true)));

        // Slave OK
        mock_conn
            .expect_get_slave()
            .returning(|_| Some(online_connection("SLAVE_1", EaType::Slave, true)));

        let updater = create_updater(mock_repo, mock_conn);

        let target = SlaveRuntimeTarget {
            master_account: "MASTER_1",
            trade_group_id: "MASTER_1",
            slave_account: "SLAVE_1",
            enabled_flag: true,
            slave_settings: &SlaveSettings::default(),
        };

        let bundle = updater.build_slave_bundle(target).await;

        assert_eq!(bundle.config.master_account, "MASTER_1");
        assert_eq!(bundle.config.account_id, "SLAVE_1");
        assert_eq!(bundle.status_result.status, 2);
    }

    #[test]
    fn snapshot_reflects_accumulated_counts() {
        let metrics = RuntimeStatusMetrics::default();

        metrics.record_master_eval_success();
        metrics.record_master_eval_failure();
        metrics.record_slave_eval_success();
        metrics.record_slave_eval_failure();
        metrics.record_slave_bundle(5);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.master_evaluations_total, 2);
        assert_eq!(snapshot.master_evaluations_failed, 1);
        assert_eq!(snapshot.slave_evaluations_total, 2);
        assert_eq!(snapshot.slave_evaluations_failed, 1);
        assert_eq!(snapshot.slave_bundles_built, 1);
        assert_eq!(snapshot.last_cluster_size, 5);
    }
}
