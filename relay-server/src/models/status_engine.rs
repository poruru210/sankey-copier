// relay-server/src/models/status_engine.rs
//
// Next-generation status evaluation engine for Master/Slave EAs.
// Provides a single source of truth for status and allow_new_orders logic.

use super::{ConnectionStatus, STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED};

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
}

impl MasterClusterSnapshot {
    pub fn new(master_statuses: Vec<i32>) -> Self {
        Self { master_statuses }
    }

    /// Returns true when every master is CONNECTED (and at least one master exists).
    pub fn all_connected(&self) -> bool {
        !self.master_statuses.is_empty()
            && self
                .master_statuses
                .iter()
                .all(|status| *status == STATUS_CONNECTED)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterStatusResult {
    pub status: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlaveStatusResult {
    pub status: i32,
    pub allow_new_orders: bool,
}

pub fn evaluate_master_status(
    intent: MasterIntent,
    conn: ConnectionSnapshot,
) -> MasterStatusResult {
    let status = if !intent.web_ui_enabled {
        STATUS_DISABLED
    } else if !is_connection_online(conn.connection_status) {
        STATUS_DISABLED
    } else if !conn.is_trade_allowed {
        STATUS_DISABLED
    } else {
        STATUS_CONNECTED
    };

    MasterStatusResult { status }
}

pub fn evaluate_slave_status(
    intent: SlaveIntent,
    slave_conn: ConnectionSnapshot,
    mastered: MasterClusterSnapshot,
) -> SlaveStatusResult {
    let status = if !intent.web_ui_enabled {
        STATUS_DISABLED
    } else if !is_connection_online(slave_conn.connection_status) {
        STATUS_DISABLED
    } else if !slave_conn.is_trade_allowed {
        STATUS_DISABLED
    } else if mastered.all_connected() {
        STATUS_CONNECTED
    } else {
        STATUS_ENABLED
    };

    SlaveStatusResult {
        status,
        allow_new_orders: status == STATUS_CONNECTED,
    }
}

fn is_connection_online(status: Option<ConnectionStatus>) -> bool {
    matches!(status, Some(ConnectionStatus::Online))
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
    }
}
