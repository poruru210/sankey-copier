// relay-server/src/models/status.rs
//
// Status calculation logic for Master and Slave EAs.
// Centralizes all status determination logic for testability.
//
// Status values:
// - 0 = DISABLED
// - 1 = ENABLED (Slave only: self is OK but Master is not CONNECTED)
// - 2 = CONNECTED
// - -1 = NO_CONFIG (used for removal/reset)

// Use status constants from trade_group_member.rs
use super::{STATUS_CONNECTED, STATUS_DISABLED, STATUS_ENABLED};

/// Input parameters for Master status calculation
#[derive(Debug, Clone)]
pub struct MasterStatusInput {
    /// Web UI Switch state (from MasterSettings.enabled)
    pub web_ui_enabled: bool,
    /// Auto-trading enabled on MT5 terminal (from Heartbeat.is_trade_allowed)
    pub is_trade_allowed: bool,
}

/// Input parameters for Slave status calculation
#[derive(Debug, Clone)]
pub struct SlaveStatusInput {
    /// Web UI Switch state for this Slave (derived from status > 0)
    pub web_ui_enabled: bool,
    /// Auto-trading enabled on Slave's MT5 terminal (from Heartbeat.is_trade_allowed)
    pub is_trade_allowed: bool,
    /// Master's calculated status (from calculate_master_status)
    pub master_status: i32,
}

/// Calculate effective status for Master EA
///
/// Master has only two states:
/// - DISABLED (0): Web UI Switch OFF or auto-trade OFF
/// - CONNECTED (2): Both conditions met
///
/// Note: Master does NOT have ENABLED state.
pub fn calculate_master_status(input: &MasterStatusInput) -> i32 {
    if !input.web_ui_enabled || !input.is_trade_allowed {
        STATUS_DISABLED
    } else {
        STATUS_CONNECTED
    }
}

/// Calculate effective status for Slave EA
///
/// Slave has three states:
/// - DISABLED (0): Own Web UI Switch OFF or own auto-trade OFF
/// - ENABLED (1): Self is OK but Master is not CONNECTED
/// - CONNECTED (2): All conditions met
pub fn calculate_slave_status(input: &SlaveStatusInput) -> i32 {
    // Priority 1: Own settings check
    if !input.web_ui_enabled || !input.is_trade_allowed {
        return STATUS_DISABLED;
    }

    // Priority 2: Master status check
    if input.master_status != STATUS_CONNECTED {
        return STATUS_ENABLED;
    }

    // All conditions met
    STATUS_CONNECTED
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Master Status Tests
    // =========================================================================

    #[test]
    fn test_master_disabled_when_web_ui_off() {
        let input = MasterStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: true,
        };
        assert_eq!(calculate_master_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_master_disabled_when_auto_trade_off() {
        let input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: false,
        };
        assert_eq!(calculate_master_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_master_disabled_when_both_off() {
        let input = MasterStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: false,
        };
        assert_eq!(calculate_master_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_master_connected_when_all_enabled() {
        let input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
        };
        assert_eq!(calculate_master_status(&input), STATUS_CONNECTED);
    }

    // =========================================================================
    // Slave Status Tests - DISABLED cases
    // =========================================================================

    #[test]
    fn test_slave_disabled_when_web_ui_off() {
        let input = SlaveStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: true,
            master_status: STATUS_CONNECTED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_slave_disabled_when_auto_trade_off() {
        let input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: false,
            master_status: STATUS_CONNECTED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_slave_disabled_when_both_off() {
        let input = SlaveStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: false,
            master_status: STATUS_CONNECTED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_DISABLED);
    }

    #[test]
    fn test_slave_disabled_takes_priority_over_master_status() {
        // Even if Master is DISABLED, Slave's own DISABLED takes priority
        let input = SlaveStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: true,
            master_status: STATUS_DISABLED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_DISABLED);
    }

    // =========================================================================
    // Slave Status Tests - ENABLED cases (Master not CONNECTED)
    // =========================================================================

    #[test]
    fn test_slave_enabled_when_master_disabled() {
        let input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status: STATUS_DISABLED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_ENABLED);
    }

    #[test]
    fn test_slave_enabled_when_master_enabled() {
        // Master ENABLED means Master is also waiting (shouldn't happen for Master, but test anyway)
        let input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status: STATUS_ENABLED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_ENABLED);
    }

    // =========================================================================
    // Slave Status Tests - CONNECTED case
    // =========================================================================

    #[test]
    fn test_slave_connected_when_all_conditions_met() {
        let input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status: STATUS_CONNECTED,
        };
        assert_eq!(calculate_slave_status(&input), STATUS_CONNECTED);
    }

    // =========================================================================
    // Edge Cases and Priority Tests
    // =========================================================================

    #[test]
    fn test_slave_status_priority_disabled_over_enabled() {
        // When Slave's own conditions are not met, it should be DISABLED
        // regardless of Master's status
        for master_status in [STATUS_DISABLED, STATUS_ENABLED, STATUS_CONNECTED] {
            let input = SlaveStatusInput {
                web_ui_enabled: false,
                is_trade_allowed: true,
                master_status,
            };
            assert_eq!(
                calculate_slave_status(&input),
                STATUS_DISABLED,
                "Slave should be DISABLED when web_ui_enabled=false, regardless of master_status={}",
                master_status
            );
        }
    }

    #[test]
    fn test_slave_status_priority_enabled_over_connected() {
        // When Master is not CONNECTED, Slave should be ENABLED (not CONNECTED)
        for master_status in [STATUS_DISABLED, STATUS_ENABLED] {
            let input = SlaveStatusInput {
                web_ui_enabled: true,
                is_trade_allowed: true,
                master_status,
            };
            assert_eq!(
                calculate_slave_status(&input),
                STATUS_ENABLED,
                "Slave should be ENABLED when master_status={} (not CONNECTED)",
                master_status
            );
        }
    }

    // =========================================================================
    // Integration-like Tests (complete scenario)
    // =========================================================================

    #[test]
    fn test_scenario_master_and_slave_both_working() {
        // Master: Web UI ON, auto-trade ON -> CONNECTED
        let master_input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
        };
        let master_status = calculate_master_status(&master_input);
        assert_eq!(master_status, STATUS_CONNECTED);

        // Slave: Web UI ON, auto-trade ON, Master CONNECTED -> CONNECTED
        let slave_input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status,
        };
        assert_eq!(calculate_slave_status(&slave_input), STATUS_CONNECTED);
    }

    #[test]
    fn test_scenario_master_auto_trade_off() {
        // Master: Web UI ON, auto-trade OFF -> DISABLED
        let master_input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: false,
        };
        let master_status = calculate_master_status(&master_input);
        assert_eq!(master_status, STATUS_DISABLED);

        // Slave: Web UI ON, auto-trade ON, Master DISABLED -> ENABLED
        let slave_input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status,
        };
        assert_eq!(calculate_slave_status(&slave_input), STATUS_ENABLED);
    }

    #[test]
    fn test_scenario_master_web_ui_off() {
        // Master: Web UI OFF, auto-trade ON -> DISABLED
        let master_input = MasterStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: true,
        };
        let master_status = calculate_master_status(&master_input);
        assert_eq!(master_status, STATUS_DISABLED);

        // Slave: Web UI ON, auto-trade ON, Master DISABLED -> ENABLED
        let slave_input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
            master_status,
        };
        assert_eq!(calculate_slave_status(&slave_input), STATUS_ENABLED);
    }

    #[test]
    fn test_scenario_slave_auto_trade_off_while_master_ok() {
        // Master: All OK -> CONNECTED
        let master_input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
        };
        let master_status = calculate_master_status(&master_input);
        assert_eq!(master_status, STATUS_CONNECTED);

        // Slave: auto-trade OFF -> DISABLED (even if Master is CONNECTED)
        let slave_input = SlaveStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: false,
            master_status,
        };
        assert_eq!(calculate_slave_status(&slave_input), STATUS_DISABLED);
    }

    #[test]
    fn test_scenario_slave_web_ui_off_while_master_ok() {
        // Master: All OK -> CONNECTED
        let master_input = MasterStatusInput {
            web_ui_enabled: true,
            is_trade_allowed: true,
        };
        let master_status = calculate_master_status(&master_input);
        assert_eq!(master_status, STATUS_CONNECTED);

        // Slave: Web UI OFF -> DISABLED (even if Master is CONNECTED)
        let slave_input = SlaveStatusInput {
            web_ui_enabled: false,
            is_trade_allowed: true,
            master_status,
        };
        assert_eq!(calculate_slave_status(&slave_input), STATUS_DISABLED);
    }
}
