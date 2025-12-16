use serde::{Deserialize, Serialize};

use crate::domain::models::{EaConnection, TradeGroup, TradeGroupMember};

/// Complete system state snapshot broadcast to connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStateSnapshot {
    pub connections: Vec<EaConnection>,
    pub trade_groups: Vec<TradeGroup>,
    pub members: Vec<TradeGroupMember>,
}
