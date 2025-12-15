use crate::models::{TradeGroup, TradeGroupMember, TradeSignal};
use anyhow::Result;
use async_trait::async_trait;

/// Abstract access to trade configuration data
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TradeRepository: Send + Sync {
    /// Get TradeGroup (Master settings) by ID
    async fn get_trade_group(&self, account_id: &str) -> Result<Option<TradeGroup>>;
    /// Get all members (Slaves) for a given master account
    async fn get_members(&self, account_id: &str) -> Result<Vec<TradeGroupMember>>;
}

/// Abstract signal publishing
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SignalPublisher: Send + Sync {
    /// Send a transformed trade signal to a specific slave associated with a master
    async fn send_trade_signal(
        &self,
        master_id: &str,
        slave_id: &str,
        signal: &TradeSignal,
    ) -> Result<()>;
}

/// Abstract operation notifications (WebSocket, logs, etc.)
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OperationNotifier: Send + Sync {
    /// Notify that a trade signal was received
    async fn notify_received(&self, signal: &TradeSignal);
    /// Notify that a trade signal was successfully copied
    async fn notify_copied(&self, slave_id: &str, signal: &TradeSignal, member_id: &str);
}
