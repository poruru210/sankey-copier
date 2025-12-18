use serde::{Deserialize, Serialize};

use crate::domain::models::{MasterSettings, SlaveSettings, TradeGroup, WarningCode};
use crate::domain::services::status_calculator::MasterStatusResult;

/// API response view that augments TradeGroup with runtime status evaluated by the status engine.
#[derive(Debug, Clone, Serialize)]
pub struct TradeGroupRuntimeView {
    pub id: String,
    pub master_settings: MasterSettings,
    pub master_runtime_status: i32,
    pub master_warning_codes: Vec<WarningCode>,
    pub created_at: String,
    pub updated_at: String,
}

impl TradeGroupRuntimeView {
    pub fn new(trade_group: TradeGroup, master_runtime: MasterStatusResult) -> Self {
        Self {
            id: trade_group.id,
            master_settings: trade_group.master_settings,
            master_runtime_status: master_runtime.status,
            master_warning_codes: master_runtime.warning_codes,
            created_at: trade_group.created_at,
            updated_at: trade_group.updated_at,
        }
    }
}

/// Request body for explicit Creation of a TradeGroup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTradeGroupRequest {
    pub id: String,
    #[serde(default)]
    pub master_settings: MasterSettings,
    /// Optional list of initial members to add atomically
    #[serde(default)]
    pub members: Vec<AddMemberRequest>,
}

/// Request body for toggling Master enabled state
#[derive(Debug, serde::Deserialize)]
pub struct ToggleMasterRequest {
    pub enabled: bool,
}

/// Request body for adding a new member to a TradeGroup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub slave_account: String,
    #[serde(default)]
    pub slave_settings: SlaveSettings,
    /// Initial enabled state (true/false) in UI terms.
    /// Backend maps this to status=2 (Enabled) or status=0 (Disabled).
    /// Default: false (Disabled) for safety
    #[serde(default = "default_false")]
    pub enabled: bool,
}

fn default_false() -> bool {
    false
}

/// Request body for toggling member status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleStatusRequest {
    pub enabled: bool,
}
