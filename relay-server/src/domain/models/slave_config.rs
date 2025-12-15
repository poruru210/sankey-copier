use crate::domain::models::SlaveSettings;
use sankey_copier_zmq::WarningCode;
use serde::{Deserialize, Serialize};

/// Slave configuration with associated Master account information.
/// Used for config distribution to Slave EAs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfigWithMaster {
    pub master_account: String,
    pub slave_account: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub enabled_flag: bool,
    /// Detailed warning codes from the Status Engine (empty when healthy)
    #[serde(default)]
    pub warning_codes: Vec<WarningCode>,
    pub slave_settings: SlaveSettings,
}
