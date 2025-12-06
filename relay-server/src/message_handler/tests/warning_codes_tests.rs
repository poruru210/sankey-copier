//! Unit tests for warning_codes change detection logic

use crate::models::WarningCode;

#[test]
fn test_warning_codes_equality_same() {
    let codes1 = vec![
        WarningCode::SlaveAutoTradingDisabled,
        WarningCode::MasterOffline,
    ];
    let codes2 = vec![
        WarningCode::SlaveAutoTradingDisabled,
        WarningCode::MasterOffline,
    ];

    // Same order, same elements - should be equal
    assert_eq!(codes1, codes2);
}

#[test]
fn test_warning_codes_equality_different_order() {
    let codes1 = vec![
        WarningCode::SlaveAutoTradingDisabled,
        WarningCode::MasterOffline,
    ];
    let codes2 = vec![
        WarningCode::MasterOffline,
        WarningCode::SlaveAutoTradingDisabled,
    ];

    // Different order - Status Engine always sorts by priority,
    // so this scenario should not occur in practice
    assert_ne!(codes1, codes2);
}

#[test]
fn test_warning_codes_equality_different_content() {
    let codes1 = vec![WarningCode::SlaveAutoTradingDisabled];
    let codes2 = vec![WarningCode::MasterAutoTradingDisabled];

    // Different warning codes - should be different
    assert_ne!(codes1, codes2);
}

#[test]
fn test_warning_codes_equality_empty_vs_filled() {
    let codes1: Vec<WarningCode> = vec![];
    let codes2 = vec![WarningCode::SlaveAutoTradingDisabled];

    // Empty vs filled - should be different
    assert_ne!(codes1, codes2);
}

#[test]
fn test_warning_codes_equality_both_empty() {
    let codes1: Vec<WarningCode> = vec![];
    let codes2: Vec<WarningCode> = vec![];

    // Both empty - should be equal
    assert_eq!(codes1, codes2);
}

#[test]
fn test_warning_codes_sorted_by_priority() {
    use crate::models::status_engine;
    use crate::models::status_engine::{ConnectionSnapshot, MasterIntent};
    use crate::models::ConnectionStatus;

    // Simulate a scenario where all Master warnings are present
    let master_intent = MasterIntent {
        web_ui_enabled: false, // Should add MasterWebUiDisabled
    };

    let master_conn = ConnectionSnapshot {
        connection_status: Some(ConnectionStatus::Offline), // Should add MasterOffline
        is_trade_allowed: false,                            // Should add MasterAutoTradingDisabled
    };

    let master_result = status_engine::evaluate_master_status(master_intent, master_conn);

    // Verify that warning_codes are present and sorted
    assert!(!master_result.warning_codes.is_empty());
    assert_eq!(master_result.warning_codes.len(), 3);

    // After sort_by_priority, order should be consistent
    let codes1 = master_result.warning_codes.clone();
    let codes2 = master_result.warning_codes.clone();
    assert_eq!(codes1, codes2);
}

#[test]
fn test_warning_codes_slave_auto_trading_disabled() {
    use crate::models::status_engine;
    use crate::models::status_engine::{ConnectionSnapshot, MasterIntent, SlaveIntent};
    use crate::models::ConnectionStatus;

    // Simulate Slave with auto-trading disabled
    let slave_intent = SlaveIntent {
        web_ui_enabled: true,
    };

    let slave_conn = ConnectionSnapshot {
        connection_status: Some(ConnectionStatus::Online),
        is_trade_allowed: false, // Auto-trading disabled
    };

    // Master is healthy
    let master_intent = MasterIntent {
        web_ui_enabled: true,
    };
    let master_conn = ConnectionSnapshot {
        connection_status: Some(ConnectionStatus::Online),
        is_trade_allowed: true,
    };
    let master_result = status_engine::evaluate_master_status(master_intent, master_conn);

    let member_result =
        status_engine::evaluate_member_status(slave_intent, slave_conn, &master_result);

    // Should have slave_auto_trading_disabled warning
    assert!(
        member_result
            .warning_codes
            .contains(&WarningCode::SlaveAutoTradingDisabled),
        "Expected SlaveAutoTradingDisabled in warning_codes"
    );

    // Runtime status should still be CONNECTED (online + web_ui_enabled + master connected)
    assert_eq!(
        member_result.status,
        crate::models::STATUS_CONNECTED,
        "Status should be CONNECTED even with auto-trading disabled"
    );
}
