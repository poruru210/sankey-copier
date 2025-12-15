// relay-server/src/models/status_engine.rs
//
// Next-generation status evaluation engine for Master/Slave EAs.
// Provides a single source of truth for status and allow_new_orders logic.

use super::{ConnectionStatus, WarningCode, STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED};

/// User-facing intent for a Master EA (e.g., Web UI toggle)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MasterIntent {
    pub web_ui_enabled: bool,
}

/// User-facing intent for a Slave EA (e.g., Web UI toggle)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlaveIntent {
    pub web_ui_enabled: bool,
}

/// Runtime snapshot of an EA connection (online/offline + auto-trading flag)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectionSnapshot {
    pub connection_status: Option<ConnectionStatus>,
    pub is_trade_allowed: bool,
}

/// Aggregated status information about the Masters linked to a Slave.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MasterClusterSnapshot {
    pub master_statuses: Vec<i32>,
    pub master_warning_codes: Vec<Vec<WarningCode>>,
}

impl MasterClusterSnapshot {
    #[allow(dead_code)]
    pub fn new(master_statuses: Vec<i32>) -> Self {
        let warning_slots = master_statuses.iter().map(|_| Vec::new()).collect();
        Self {
            master_statuses,
            master_warning_codes: warning_slots,
        }
    }

    #[allow(dead_code)]
    pub fn with_status_results(results: Vec<MasterStatusResult>) -> Self {
        let mut statuses = Vec::with_capacity(results.len());
        let mut warnings = Vec::with_capacity(results.len());
        for result in results {
            statuses.push(result.status);
            warnings.push(result.warning_codes);
        }
        Self {
            master_statuses: statuses,
            master_warning_codes: warnings,
        }
    }

    /// Returns true when every master is CONNECTED (and at least one master exists).
    #[allow(dead_code)]
    pub fn all_connected(&self) -> bool {
        !self.master_statuses.is_empty()
            && self
                .master_statuses
                .iter()
                .all(|status| *status == STATUS_CONNECTED)
    }

    #[allow(dead_code)]
    pub fn aggregated_warning_codes(&self) -> Vec<WarningCode> {
        let mut combined = Vec::new();
        for codes in &self.master_warning_codes {
            for code in codes {
                push_warning(&mut combined, code.clone());
            }
        }
        combined
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterStatusResult {
    pub status: i32,
    pub warning_codes: Vec<WarningCode>,
}

impl Default for MasterStatusResult {
    fn default() -> Self {
        Self {
            status: STATUS_DISABLED,
            warning_codes: vec![WarningCode::MasterOffline],
        }
    }
}

impl MasterStatusResult {
    pub fn has_changed(&self, other: &Self) -> bool {
        self.status != other.status || self.warning_codes != other.warning_codes
    }

    /// Returns a special 'Unknown' state (-1)
    pub fn unknown() -> Self {
        Self {
            status: -1,
            warning_codes: Vec::new(),
        }
    }
}

/// Result for a single Member (Master-Slave connection) status evaluation.
/// Unlike SlaveStatusResult which aggregates all Masters, this evaluates
/// the status of a specific Master-Slave pair.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemberStatusResult {
    pub status: i32,
    pub allow_new_orders: bool,
    pub warning_codes: Vec<WarningCode>,
}

impl MemberStatusResult {
    /// Returns true if the status OR warning codes differ from the other result.
    /// Warning codes are assumed to be sorted by priority by `evaluate_member_status`.
    pub fn has_changed(&self, other: &Self) -> bool {
        self.status != other.status || self.warning_codes != other.warning_codes
    }

    /// Returns a special 'Unknown' state (-1)
    pub fn unknown() -> Self {
        Self {
            status: -1,
            warning_codes: Vec::new(),
            allow_new_orders: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlaveStatusResult {
    pub status: i32,
    pub allow_new_orders: bool,
    pub warning_codes: Vec<WarningCode>,
}

pub fn evaluate_master_status(
    intent: MasterIntent,
    conn: ConnectionSnapshot,
) -> MasterStatusResult {
    let mut warning_codes = Vec::new();
    let is_online = is_connection_online(conn.connection_status);

    if !intent.web_ui_enabled {
        warning_codes.push(WarningCode::MasterWebUiDisabled);
    }
    if !is_online {
        warning_codes.push(WarningCode::MasterOffline);
    }
    // Only check auto-trading state if Master is online
    // (offline Master has no valid is_trade_allowed state yet)
    if is_online && !conn.is_trade_allowed {
        warning_codes.push(WarningCode::MasterAutoTradingDisabled);
    }

    let status = if warning_codes.is_empty() {
        STATUS_CONNECTED
    } else {
        STATUS_DISABLED
    };

    MasterStatusResult {
        status,
        warning_codes,
    }
}

#[allow(dead_code)]
pub fn evaluate_slave_status(
    intent: SlaveIntent,
    slave_conn: ConnectionSnapshot,
    mastered: MasterClusterSnapshot,
) -> SlaveStatusResult {
    let mut warning_codes = Vec::new();

    // Core conditions for Slave to be operational
    let slave_web_ui_enabled = intent.web_ui_enabled;
    let slave_online = is_connection_online(slave_conn.connection_status);

    if !slave_web_ui_enabled {
        warning_codes.push(WarningCode::SlaveWebUiDisabled);
    }
    if !slave_online {
        warning_codes.push(WarningCode::SlaveOffline);
    }
    // Only check auto-trading state if Slave is online
    // (offline Slave has no valid is_trade_allowed state yet)
    if slave_online && !slave_conn.is_trade_allowed {
        warning_codes.push(WarningCode::SlaveAutoTradingDisabled);
    }

    // Slave is DISABLED if Web UI is OFF, Slave is offline, or (online but auto-trading disabled)
    // Otherwise, status depends on Master cluster state (for display purposes)
    let slave_disabled =
        !slave_web_ui_enabled || !slave_online || (slave_online && !slave_conn.is_trade_allowed);

    let mut status = if slave_disabled {
        STATUS_DISABLED
    } else if mastered.all_connected() {
        STATUS_CONNECTED
    } else {
        STATUS_ENABLED
    };

    if status != STATUS_DISABLED {
        if mastered.master_statuses.is_empty() {
            push_warning(&mut warning_codes, WarningCode::NoMasterAssigned);
            status = STATUS_ENABLED;
        } else if !mastered.all_connected() {
            push_warning(&mut warning_codes, WarningCode::MasterClusterDegraded);
            for code in mastered.aggregated_warning_codes() {
                push_warning(&mut warning_codes, code);
            }
        }
    }

    // allow_new_orders: Slave can process signals only when fully operational
    // (Web UI ON, online, and auto-trading enabled)
    let allow_new_orders = slave_web_ui_enabled && slave_online && slave_conn.is_trade_allowed;

    SlaveStatusResult {
        status,
        allow_new_orders,
        warning_codes,
    }
}

/// Evaluate the status of a single Member (Master-Slave connection).
///
/// Unlike `evaluate_slave_status` which takes a cluster of all Masters,
/// this function evaluates the status based on a single Master's state.
/// This allows each connection to have its own independent status.
///
/// # Arguments
/// * `intent` - User's intent for this Slave (Web UI toggle)
/// * `slave_conn` - Slave's connection snapshot (online/offline, is_trade_allowed)
/// * `master_result` - The specific Master's status result
///
/// # Returns
/// `MemberStatusResult` containing:
/// - `status`: 0=DISABLED, 1=ENABLED, 2=CONNECTED
/// - `allow_new_orders`: Whether the Slave can process new trade signals
/// - `warning_codes`: Detailed reasons for non-CONNECTED status
pub fn evaluate_member_status(
    intent: SlaveIntent,
    slave_conn: ConnectionSnapshot,
    master_result: &MasterStatusResult,
) -> MemberStatusResult {
    let mut warning_codes = Vec::new();

    // Core conditions for Slave to be operational
    let slave_web_ui_enabled = intent.web_ui_enabled;
    let slave_online = is_connection_online(slave_conn.connection_status);

    // Collect Slave-side warnings
    if !slave_web_ui_enabled {
        warning_codes.push(WarningCode::SlaveWebUiDisabled);
    }
    if !slave_online {
        warning_codes.push(WarningCode::SlaveOffline);
    }
    // Only check auto-trading state if Slave is online
    // (offline Slave has no valid is_trade_allowed state yet)
    if slave_online && !slave_conn.is_trade_allowed {
        warning_codes.push(WarningCode::SlaveAutoTradingDisabled);
    }

    // Slave is DISABLED if Web UI is OFF, Slave is offline, or (online but auto-trading disabled)
    let slave_disabled =
        !slave_web_ui_enabled || !slave_online || (slave_online && !slave_conn.is_trade_allowed);

    // Always include Master's warning codes regardless of Slave status
    // (Users need to see Master issues even when Slave is offline)
    for code in &master_result.warning_codes {
        push_warning(&mut warning_codes, code.clone());
    }

    // Determine status based on Slave and Master state
    let status = if slave_disabled {
        STATUS_DISABLED
    } else if master_result.status == STATUS_CONNECTED {
        // Master is healthy, this connection is CONNECTED
        STATUS_CONNECTED
    } else {
        // Master is not connected, this connection is ENABLED (waiting)
        STATUS_ENABLED
    };

    // allow_new_orders: Slave can process signals only when fully operational
    // (Web UI ON, online, and auto-trading enabled)
    let allow_new_orders = slave_web_ui_enabled && slave_online && slave_conn.is_trade_allowed;

    // Sort warning codes by priority for consistent display
    WarningCode::sort_by_priority(&mut warning_codes);

    MemberStatusResult {
        status,
        allow_new_orders,
        warning_codes,
    }
}

fn is_connection_online(status: Option<ConnectionStatus>) -> bool {
    matches!(status, Some(ConnectionStatus::Online))
}

fn push_warning(vec: &mut Vec<WarningCode>, code: WarningCode) {
    if !vec.contains(&code) {
        vec.push(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_disabled_when_web_ui_off() {
        let intent = MasterIntent {
            web_ui_enabled: false,
        };
        let conn = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        };
        assert_eq!(evaluate_master_status(intent, conn).status, STATUS_DISABLED);
    }

    #[test]
    fn master_connected_when_all_ok() {
        let intent = MasterIntent {
            web_ui_enabled: true,
        };
        let conn = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        };
        assert_eq!(
            evaluate_master_status(intent, conn).status,
            STATUS_CONNECTED
        );
    }

    #[test]
    fn slave_disabled_when_auto_trade_off() {
        // is_trade_allowed=false now DISABLES slave (status=0, allow_new_orders=false)
        // This ensures panel shows DISABLED when MT auto-trading is OFF
        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: true,
            },
            ConnectionSnapshot {
                connection_status: Some(ConnectionStatus::Online),
                is_trade_allowed: false,
            },
            MasterClusterSnapshot::new(vec![STATUS_CONNECTED]),
        );
        // Status is now DISABLED because auto-trading is OFF
        assert_eq!(result.status, STATUS_DISABLED);
        // allow_new_orders is false because auto-trading is OFF
        assert!(!result.allow_new_orders);
        assert!(result
            .warning_codes
            .contains(&WarningCode::SlaveAutoTradingDisabled));
    }

    #[test]
    fn slave_connected_when_all_masters_connected() {
        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: true,
            },
            ConnectionSnapshot {
                connection_status: Some(ConnectionStatus::Online),
                is_trade_allowed: true,
            },
            MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_CONNECTED]),
        );
        assert_eq!(result.status, STATUS_CONNECTED);
        assert!(result.allow_new_orders);
        assert!(result.warning_codes.is_empty());
    }

    #[test]
    fn slave_enabled_when_any_master_disabled() {
        // Slave status reflects Master cluster state for display purposes
        // but allow_new_orders depends only on Slave's own state
        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: true,
            },
            ConnectionSnapshot {
                connection_status: Some(ConnectionStatus::Online),
                is_trade_allowed: true,
            },
            MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_ENABLED]),
        );
        assert_eq!(result.status, STATUS_ENABLED);
        // allow_new_orders is true because Slave's Web UI is ON and Slave is online
        // Master's connection state does NOT affect this
        assert!(result.allow_new_orders);
        assert!(result
            .warning_codes
            .contains(&WarningCode::MasterClusterDegraded));
    }

    #[test]
    fn master_cluster_requires_non_empty_for_all_connected() {
        let empty_cluster = MasterClusterSnapshot::default();
        assert!(!empty_cluster.all_connected());

        let mixed_cluster = MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_ENABLED]);
        assert!(!mixed_cluster.all_connected());

        let healthy_cluster = MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_CONNECTED]);
        assert!(healthy_cluster.all_connected());
    }

    #[test]
    fn slave_enabled_when_no_master_connection_yet() {
        // Slave can still allow orders even without Master assigned
        // If signals somehow arrive, they should be processed
        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: true,
            },
            ConnectionSnapshot {
                connection_status: Some(ConnectionStatus::Online),
                is_trade_allowed: true,
            },
            MasterClusterSnapshot::default(),
        );

        assert_eq!(result.status, STATUS_ENABLED);
        // allow_new_orders is true because Slave's Web UI is ON and Slave is online
        assert!(result.allow_new_orders);
        assert!(result
            .warning_codes
            .contains(&WarningCode::NoMasterAssigned));
    }

    #[test]
    fn slave_disabled_when_connection_offline() {
        let result = evaluate_slave_status(
            SlaveIntent {
                web_ui_enabled: true,
            },
            ConnectionSnapshot {
                connection_status: Some(ConnectionStatus::Offline),
                is_trade_allowed: true,
            },
            MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_CONNECTED]),
        );

        assert_eq!(result.status, STATUS_DISABLED);
        assert!(!result.allow_new_orders);
        assert!(result.warning_codes.contains(&WarningCode::SlaveOffline));
    }

    #[test]
    fn master_cluster_aggregates_unique_warning_codes() {
        let snapshot = MasterClusterSnapshot::with_status_results(vec![
            MasterStatusResult {
                status: STATUS_DISABLED,
                warning_codes: vec![WarningCode::MasterOffline, WarningCode::MasterWebUiDisabled],
            },
            MasterStatusResult {
                status: STATUS_DISABLED,
                warning_codes: vec![
                    WarningCode::MasterOffline,
                    WarningCode::MasterAutoTradingDisabled,
                ],
            },
        ]);

        let aggregated = snapshot.aggregated_warning_codes();
        assert_eq!(aggregated.len(), 3);
        assert!(aggregated.contains(&WarningCode::MasterOffline));
        assert!(aggregated.contains(&WarningCode::MasterWebUiDisabled));
        assert!(aggregated.contains(&WarningCode::MasterAutoTradingDisabled));
    }

    #[test]
    fn master_status_combinations_cover_all_inputs() {
        #[derive(Clone, Copy)]
        struct MasterCase {
            name: &'static str,
            intent_enabled: bool,
            connection: Option<ConnectionStatus>,
            is_trade_allowed: bool,
            expected_status: i32,
            expected_warnings: &'static [WarningCode],
        }

        use ConnectionStatus::{Offline, Online};

        let cases = [
            MasterCase {
                name: "all_green",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_CONNECTED,
                expected_warnings: &[],
            },
            MasterCase {
                name: "offline_only",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterOffline],
            },
            MasterCase {
                name: "unknown_connection",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterOffline],
            },
            MasterCase {
                name: "trade_blocked",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterAutoTradingDisabled],
            },
            MasterCase {
                name: "offline_and_trade_blocked",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                // Offline: is_trade_allowed is not checked (no valid state yet)
                expected_warnings: &[WarningCode::MasterOffline],
            },
            MasterCase {
                name: "unknown_and_trade_blocked",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                // Unknown/offline: is_trade_allowed is not checked (no valid state yet)
                expected_warnings: &[WarningCode::MasterOffline],
            },
            MasterCase {
                name: "intent_off_only",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterWebUiDisabled],
            },
            MasterCase {
                name: "intent_off_and_trade_blocked",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::MasterWebUiDisabled,
                    WarningCode::MasterAutoTradingDisabled,
                ],
            },
            MasterCase {
                name: "intent_off_and_offline",
                intent_enabled: false,
                connection: Some(Offline),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterWebUiDisabled, WarningCode::MasterOffline],
            },
            MasterCase {
                name: "intent_off_offline_blocked",
                intent_enabled: false,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                // Offline: is_trade_allowed is not checked
                expected_warnings: &[WarningCode::MasterWebUiDisabled, WarningCode::MasterOffline],
            },
            MasterCase {
                name: "intent_off_unknown",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::MasterWebUiDisabled, WarningCode::MasterOffline],
            },
            MasterCase {
                name: "intent_off_unknown_blocked",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                // Unknown/offline: is_trade_allowed is not checked
                expected_warnings: &[WarningCode::MasterWebUiDisabled, WarningCode::MasterOffline],
            },
        ];

        for case in cases {
            let result = evaluate_master_status(
                MasterIntent {
                    web_ui_enabled: case.intent_enabled,
                },
                ConnectionSnapshot {
                    connection_status: case.connection,
                    is_trade_allowed: case.is_trade_allowed,
                },
            );

            assert_eq!(result.status, case.expected_status, "case {}", case.name);
            assert_eq!(
                result.warning_codes, case.expected_warnings,
                "case {}",
                case.name
            );
        }
    }

    #[test]
    fn slave_status_local_combinations_cover_all_inputs() {
        #[derive(Clone, Copy)]
        struct SlaveCase {
            name: &'static str,
            intent_enabled: bool,
            connection: Option<ConnectionStatus>,
            is_trade_allowed: bool,
            expected_status: i32,
            expected_allow_new_orders: bool,
            expected_warnings: &'static [WarningCode],
        }

        use ConnectionStatus::{Offline, Online};
        let healthy_cluster = MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_CONNECTED]);

        // allow_new_orders = web_ui_enabled && online && is_trade_allowed
        // is_trade_allowed=false now causes DISABLED status
        let cases = [
            SlaveCase {
                name: "all_green",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_CONNECTED,
                expected_allow_new_orders: true,
                expected_warnings: &[],
            },
            SlaveCase {
                name: "trade_blocked",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED, // trade_blocked now disables
                expected_allow_new_orders: false, // Slave cannot process signals when auto-trading OFF
                expected_warnings: &[WarningCode::SlaveAutoTradingDisabled],
            },
            SlaveCase {
                name: "offline_only",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "offline_and_trade_blocked",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                // Offline: is_trade_allowed is not checked (no valid state yet)
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "unknown_connection",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "unknown_and_trade_blocked",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                // Unknown/offline: is_trade_allowed is not checked (no valid state yet)
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_only",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[WarningCode::SlaveWebUiDisabled],
            },
            SlaveCase {
                name: "intent_off_and_trade_blocked",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[
                    WarningCode::SlaveWebUiDisabled,
                    WarningCode::SlaveAutoTradingDisabled,
                ],
            },
            SlaveCase {
                name: "intent_off_and_offline",
                intent_enabled: false,
                connection: Some(Offline),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_offline_blocked",
                intent_enabled: false,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                // Web UI OFF + offline: is_trade_allowed is not checked (offline)
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_unknown",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_unknown_blocked",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_allow_new_orders: false,
                // Web UI OFF + unknown: is_trade_allowed is not checked (offline)
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
        ];

        for case in cases {
            let result = evaluate_slave_status(
                SlaveIntent {
                    web_ui_enabled: case.intent_enabled,
                },
                ConnectionSnapshot {
                    connection_status: case.connection,
                    is_trade_allowed: case.is_trade_allowed,
                },
                healthy_cluster.clone(),
            );

            assert_eq!(result.status, case.expected_status, "case {}", case.name);
            assert_eq!(
                result.allow_new_orders, case.expected_allow_new_orders,
                "case {} allow_new_orders",
                case.name
            );
            assert_eq!(
                result.warning_codes, case.expected_warnings,
                "case {}",
                case.name
            );
        }
    }

    #[test]
    fn slave_status_reflects_master_cluster_variants() {
        use ConnectionStatus::Online;
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let snapshot = ConnectionSnapshot {
            connection_status: Some(Online),
            is_trade_allowed: true,
        };

        let healthy_cluster = MasterClusterSnapshot::new(vec![STATUS_CONNECTED]);
        let degraded_cluster = MasterClusterSnapshot::with_status_results(vec![
            MasterStatusResult {
                status: STATUS_CONNECTED,
                warning_codes: vec![],
            },
            MasterStatusResult {
                status: STATUS_ENABLED,
                warning_codes: vec![WarningCode::MasterOffline],
            },
        ]);
        let empty_cluster = MasterClusterSnapshot::default();

        let healthy = evaluate_slave_status(intent, snapshot, healthy_cluster);
        assert_eq!(healthy.status, STATUS_CONNECTED);
        assert!(healthy.allow_new_orders);
        assert!(healthy.warning_codes.is_empty());

        let degraded = evaluate_slave_status(intent, snapshot, degraded_cluster);
        assert_eq!(degraded.status, STATUS_ENABLED);
        // allow_new_orders is true because Slave's Web UI is ON and Slave is online
        // Master cluster being degraded does NOT block orders
        assert!(degraded.allow_new_orders);
        assert_eq!(
            degraded.warning_codes,
            &[
                WarningCode::MasterClusterDegraded,
                WarningCode::MasterOffline
            ]
        );

        let orphaned = evaluate_slave_status(intent, snapshot, empty_cluster);
        assert_eq!(orphaned.status, STATUS_ENABLED);
        // allow_new_orders is true - if signals arrive somehow, process them
        assert!(orphaned.allow_new_orders);
        assert_eq!(orphaned.warning_codes, &[WarningCode::NoMasterAssigned]);
    }

    // ========================================
    // Tests for evaluate_member_status (per-connection)
    // ========================================

    #[test]
    fn member_connected_when_master_connected_and_slave_ok() {
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        };
        let master_result = MasterStatusResult {
            status: STATUS_CONNECTED,
            warning_codes: vec![],
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        assert_eq!(result.status, STATUS_CONNECTED);
        assert!(result.allow_new_orders);
        assert!(result.warning_codes.is_empty());
    }

    #[test]
    fn member_enabled_when_master_offline() {
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        };
        let master_result = MasterStatusResult {
            status: STATUS_DISABLED,
            warning_codes: vec![WarningCode::MasterOffline],
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        assert_eq!(result.status, STATUS_ENABLED);
        // allow_new_orders is true - Slave can process signals if they arrive
        assert!(result.allow_new_orders);
        // Master's warning code should be propagated
        assert!(result.warning_codes.contains(&WarningCode::MasterOffline));
    }

    #[test]
    fn member_disabled_when_slave_offline() {
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Offline),
            is_trade_allowed: true,
        };
        let master_result = MasterStatusResult {
            status: STATUS_CONNECTED,
            warning_codes: vec![],
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        assert_eq!(result.status, STATUS_DISABLED);
        assert!(!result.allow_new_orders);
        assert!(result.warning_codes.contains(&WarningCode::SlaveOffline));
    }

    #[test]
    fn member_disabled_when_slave_web_ui_off() {
        let intent = SlaveIntent {
            web_ui_enabled: false,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: true,
        };
        let master_result = MasterStatusResult {
            status: STATUS_CONNECTED,
            warning_codes: vec![],
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        assert_eq!(result.status, STATUS_DISABLED);
        assert!(!result.allow_new_orders);
        assert!(result
            .warning_codes
            .contains(&WarningCode::SlaveWebUiDisabled));
    }

    #[test]
    fn member_warning_codes_sorted_by_priority() {
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: false, // generates SlaveAutoTradingDisabled
        };
        let master_result = MasterStatusResult {
            status: STATUS_DISABLED,
            warning_codes: vec![WarningCode::MasterOffline], // priority 50
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        // SlaveAutoTradingDisabled (priority 30) should come before MasterOffline (priority 50)
        assert_eq!(result.warning_codes.len(), 2);
        assert_eq!(
            result.warning_codes[0],
            WarningCode::SlaveAutoTradingDisabled
        );
        assert_eq!(result.warning_codes[1], WarningCode::MasterOffline);
    }

    #[test]
    fn member_disabled_when_slave_auto_trading_off() {
        // Slave is DISABLED when online but auto-trading is off
        // This prevents trade execution when MetaTrader's auto-trading is disabled
        let intent = SlaveIntent {
            web_ui_enabled: true,
        };
        let slave_snapshot = ConnectionSnapshot {
            connection_status: Some(ConnectionStatus::Online),
            is_trade_allowed: false,
        };
        let master_result = MasterStatusResult {
            status: STATUS_CONNECTED,
            warning_codes: vec![],
        };

        let result = evaluate_member_status(intent, slave_snapshot, &master_result);
        assert_eq!(
            result.status, STATUS_DISABLED,
            "Slave should be DISABLED when auto-trading is off"
        );
        assert!(
            !result.allow_new_orders,
            "allow_new_orders should be false when auto-trading is off"
        );
        assert!(result
            .warning_codes
            .contains(&WarningCode::SlaveAutoTradingDisabled));
    }
    #[test]
    fn test_has_changed_logic() {
        let base = MemberStatusResult {
            status: STATUS_CONNECTED,
            allow_new_orders: true,
            warning_codes: vec![],
        };

        // Case 1: Identical -> false
        let identical = base.clone();
        assert!(!base.has_changed(&identical));

        // Case 2: Status different -> true
        let status_diff = MemberStatusResult {
            status: STATUS_DISABLED,
            ..base.clone()
        };
        assert!(base.has_changed(&status_diff));

        // Case 3: Warning codes content different -> true
        let warning_diff = MemberStatusResult {
            warning_codes: vec![WarningCode::SlaveOffline],
            ..base.clone()
        };
        assert!(base.has_changed(&warning_diff));

        // Case 4: Warning codes count different -> true
        let warning_count_diff = MemberStatusResult {
            warning_codes: vec![WarningCode::SlaveOffline, WarningCode::MasterOffline],
            ..base.clone()
        };
        assert!(base.has_changed(&warning_count_diff));

        // Case 5: Warning codes order different (but same content) -> true
        // Note: In practice, evaluate_member_status sorts them, so this shouldn't happen naturally,
        // but has_changed should strictly compare vectors.
        let warning_order_a = MemberStatusResult {
            warning_codes: vec![WarningCode::SlaveOffline, WarningCode::MasterOffline],
            ..base.clone()
        };
        let warning_order_b = MemberStatusResult {
            warning_codes: vec![WarningCode::MasterOffline, WarningCode::SlaveOffline],
            ..base.clone()
        };
        assert!(warning_order_a.has_changed(&warning_order_b));
    }

    #[test]
    fn test_unknown_state_logic() {
        // TDD: Define expected behavior for "Unknown" state
        // 1. Unknown should be different from Default (Disabled)
        // 2. Unknown should be different from Connected
        // 3. Unknown should be equal to Unknown

        // This will fail to compile initially because unknown() is not defined
        let unknown = MemberStatusResult::unknown();
        let default = MemberStatusResult::default(); // usually status=0 (Disabled)
        let connected = MemberStatusResult {
            status: crate::models::STATUS_CONNECTED,
            warning_codes: vec![],
            allow_new_orders: true,
        };

        // Unknown vs Default
        assert!(unknown.has_changed(&default));
        assert!(default.has_changed(&unknown));

        // Unknown vs Connected
        assert!(unknown.has_changed(&connected));
        assert!(connected.has_changed(&unknown));

        // Unknown vs Unknown
        assert!(!unknown.has_changed(&unknown));

        // Check MasterStatusResult unknown as well
        let master_unknown = MasterStatusResult::unknown();
        let master_default = MasterStatusResult::default();

        assert!(master_unknown.has_changed(&master_default));
        assert!(!master_unknown.has_changed(&master_unknown));
    }
}
