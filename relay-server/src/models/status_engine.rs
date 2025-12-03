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
    pub fn all_connected(&self) -> bool {
        !self.master_statuses.is_empty()
            && self
                .master_statuses
                .iter()
                .all(|status| *status == STATUS_CONNECTED)
    }
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

    if !intent.web_ui_enabled {
        warning_codes.push(WarningCode::MasterWebUiDisabled);
    }
    if !is_connection_online(conn.connection_status) {
        warning_codes.push(WarningCode::MasterOffline);
    }
    if !conn.is_trade_allowed {
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

pub fn evaluate_slave_status(
    intent: SlaveIntent,
    slave_conn: ConnectionSnapshot,
    mastered: MasterClusterSnapshot,
) -> SlaveStatusResult {
    let mut warning_codes = Vec::new();

    if !intent.web_ui_enabled {
        warning_codes.push(WarningCode::SlaveWebUiDisabled);
    }
    if !is_connection_online(slave_conn.connection_status) {
        warning_codes.push(WarningCode::SlaveOffline);
    }
    if !slave_conn.is_trade_allowed {
        warning_codes.push(WarningCode::SlaveAutoTradingDisabled);
    }

    let mut status = if !warning_codes.is_empty() {
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

    SlaveStatusResult {
        status,
        allow_new_orders: status == STATUS_CONNECTED,
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
        assert_eq!(result.status, STATUS_DISABLED);
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
        assert!(!result.allow_new_orders);
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
        assert!(!result.allow_new_orders);
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
                expected_warnings: &[
                    WarningCode::MasterOffline,
                    WarningCode::MasterAutoTradingDisabled,
                ],
            },
            MasterCase {
                name: "unknown_and_trade_blocked",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::MasterOffline,
                    WarningCode::MasterAutoTradingDisabled,
                ],
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
                expected_warnings: &[
                    WarningCode::MasterWebUiDisabled,
                    WarningCode::MasterOffline,
                    WarningCode::MasterAutoTradingDisabled,
                ],
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
                expected_warnings: &[
                    WarningCode::MasterWebUiDisabled,
                    WarningCode::MasterOffline,
                    WarningCode::MasterAutoTradingDisabled,
                ],
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
            expected_warnings: &'static [WarningCode],
        }

        use ConnectionStatus::{Offline, Online};
        let healthy_cluster = MasterClusterSnapshot::new(vec![STATUS_CONNECTED, STATUS_CONNECTED]);

        let cases = [
            SlaveCase {
                name: "all_green",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_CONNECTED,
                expected_warnings: &[],
            },
            SlaveCase {
                name: "trade_blocked",
                intent_enabled: true,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::SlaveAutoTradingDisabled],
            },
            SlaveCase {
                name: "offline_only",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "offline_and_trade_blocked",
                intent_enabled: true,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::SlaveOffline,
                    WarningCode::SlaveAutoTradingDisabled,
                ],
            },
            SlaveCase {
                name: "unknown_connection",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "unknown_and_trade_blocked",
                intent_enabled: true,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::SlaveOffline,
                    WarningCode::SlaveAutoTradingDisabled,
                ],
            },
            SlaveCase {
                name: "intent_off_only",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::SlaveWebUiDisabled],
            },
            SlaveCase {
                name: "intent_off_and_trade_blocked",
                intent_enabled: false,
                connection: Some(Online),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
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
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_offline_blocked",
                intent_enabled: false,
                connection: Some(Offline),
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::SlaveWebUiDisabled,
                    WarningCode::SlaveOffline,
                    WarningCode::SlaveAutoTradingDisabled,
                ],
            },
            SlaveCase {
                name: "intent_off_unknown",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: true,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[WarningCode::SlaveWebUiDisabled, WarningCode::SlaveOffline],
            },
            SlaveCase {
                name: "intent_off_unknown_blocked",
                intent_enabled: false,
                connection: None,
                is_trade_allowed: false,
                expected_status: STATUS_DISABLED,
                expected_warnings: &[
                    WarningCode::SlaveWebUiDisabled,
                    WarningCode::SlaveOffline,
                    WarningCode::SlaveAutoTradingDisabled,
                ],
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
                result.allow_new_orders,
                case.expected_status == STATUS_CONNECTED
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
        assert!(!degraded.allow_new_orders);
        assert_eq!(
            degraded.warning_codes,
            &[
                WarningCode::MasterClusterDegraded,
                WarningCode::MasterOffline
            ]
        );

        let orphaned = evaluate_slave_status(intent, snapshot, empty_cluster);
        assert_eq!(orphaned.status, STATUS_ENABLED);
        assert!(!orphaned.allow_new_orders);
        assert_eq!(orphaned.warning_codes, &[WarningCode::NoMasterAssigned]);
    }
}
