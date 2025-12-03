use std::sync::{atomic::AtomicU64, atomic::Ordering, Arc};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    config_builder::{ConfigBuilder, SlaveConfigBundle, SlaveConfigContext},
    connection_manager::ConnectionManager,
    db::Database,
    models::{
        status_engine::{
            evaluate_master_status, evaluate_slave_status, ConnectionSnapshot,
            MasterClusterSnapshot, MasterIntent, MasterStatusResult, SlaveIntent,
            SlaveStatusResult,
        },
        SlaveSettings, WarningCode, STATUS_DISABLED,
    },
};

/// Input payload describing a specific Slave connection that needs runtime evaluation.
pub struct SlaveRuntimeTarget<'a> {
    pub master_account: &'a str,
    pub slave_account: &'a str,
    pub trade_group_id: &'a str,
    pub enabled_flag: bool,
    pub slave_settings: &'a SlaveSettings,
}

/// Helper that centralizes runtime snapshot gathering for Master/Slave pairs.
#[derive(Clone)]
pub struct RuntimeStatusUpdater {
    db: Arc<Database>,
    connection_manager: Arc<ConnectionManager>,
    metrics: Arc<RuntimeStatusMetrics>,
}

impl RuntimeStatusUpdater {
    pub fn with_metrics(
        db: Arc<Database>,
        connection_manager: Arc<ConnectionManager>,
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
        let slave_conn = self.connection_manager.get_ea(slave_account).await;
        ConnectionSnapshot {
            connection_status: slave_conn.as_ref().map(|conn| conn.status),
            is_trade_allowed: slave_conn
                .as_ref()
                .map(|conn| conn.is_trade_allowed)
                .unwrap_or(false),
        }
    }

    #[instrument(skip(self), fields(slave_account = %slave_account))]
    pub async fn master_cluster_snapshot(&self, slave_account: &str) -> MasterClusterSnapshot {
        match self.db.get_masters_for_slave(slave_account).await {
            Ok(master_accounts) => {
                let mut results = Vec::with_capacity(master_accounts.len());
                for master in master_accounts {
                    let result = self
                        .evaluate_master_runtime_status(&master)
                        .await
                        .unwrap_or_else(|| MasterStatusResult {
                            status: STATUS_DISABLED,
                            warning_codes: vec![WarningCode::MasterOffline],
                        });
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

        let master_conn = self.connection_manager.get_ea(master_account).await;
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

        tracing::debug!(
            target: "runtime_status",
            master_account = %master_account,
            status = result.status,
            warning_count = result.warning_codes.len(),
            "evaluated master runtime status"
        );
        self.metrics.record_master_eval_success();
        Some(result)
    }

    #[instrument(skip(self, target, master_cluster), fields(slave_account = %target.slave_account, master_account = %target.master_account))]
    pub async fn build_slave_bundle(
        &self,
        target: SlaveRuntimeTarget<'_>,
        master_cluster: Option<MasterClusterSnapshot>,
    ) -> SlaveConfigBundle {
        let cluster = match master_cluster {
            Some(snapshot) => snapshot,
            None => self.master_cluster_snapshot(target.slave_account).await,
        };

        let slave_snapshot = self.slave_connection_snapshot(target.slave_account).await;
        let master_equity = self
            .connection_manager
            .get_ea(target.master_account)
            .await
            .map(|conn| conn.equity);

        let cluster_size = cluster.master_statuses.len();
        let bundle = ConfigBuilder::build_slave_config(SlaveConfigContext {
            slave_account: target.slave_account.to_string(),
            master_account: target.master_account.to_string(),
            trade_group_id: target.trade_group_id.to_string(),
            intent: SlaveIntent {
                web_ui_enabled: target.enabled_flag,
            },
            slave_connection_snapshot: slave_snapshot,
            master_cluster: cluster.clone(),
            slave_settings: target.slave_settings,
            master_equity,
            timestamp: Utc::now(),
        });

        tracing::debug!(
            target: "runtime_status",
            slave_account = %target.slave_account,
            master_account = %target.master_account,
            runtime_status = bundle.status_result.status,
            allow_new_orders = bundle.status_result.allow_new_orders,
            warning_count = bundle.status_result.warning_codes.len(),
            cluster_size = cluster_size,
            "built slave config bundle"
        );
        self.metrics.record_slave_bundle(cluster_size as u64);

        bundle
    }

    #[instrument(skip(self, target, master_cluster), fields(slave_account = %target.slave_account, master_account = %target.master_account))]
    pub async fn evaluate_slave_runtime_status(
        &self,
        target: SlaveRuntimeTarget<'_>,
        master_cluster: Option<MasterClusterSnapshot>,
    ) -> SlaveStatusResult {
        let cluster = match master_cluster {
            Some(snapshot) => snapshot,
            None => self.master_cluster_snapshot(target.slave_account).await,
        };

        let slave_snapshot = self.slave_connection_snapshot(target.slave_account).await;

        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: target.enabled_flag,
            },
            slave_snapshot,
            cluster,
        );

        tracing::debug!(
            target: "runtime_status",
            slave_account = %target.slave_account,
            master_account = %target.master_account,
            runtime_status = result.status,
            allow_new_orders = result.allow_new_orders,
            warning_count = result.warning_codes.len(),
            "evaluated slave runtime status"
        );
        self.metrics.record_slave_eval_success();

        result
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

    #[test]
    fn failure_helpers_increment_total_and_failed() {
        let metrics = RuntimeStatusMetrics::default();

        metrics.record_master_eval_failure();
        metrics.record_slave_eval_failure();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.master_evaluations_total, 1);
        assert_eq!(snapshot.master_evaluations_failed, 1);
        assert_eq!(snapshot.slave_evaluations_total, 1);
        assert_eq!(snapshot.slave_evaluations_failed, 1);
    }
}
