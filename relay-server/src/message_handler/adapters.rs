use crate::db::Database;
use crate::models::{TradeGroup, TradeGroupMember, TradeSignal};
use crate::ports::{OperationNotifier, SignalPublisher, TradeRepository};
use crate::zeromq::ZmqConfigPublisher;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

#[async_trait]
impl TradeRepository for Database {
    async fn get_trade_group(&self, account_id: &str) -> Result<Option<TradeGroup>> {
        self.get_trade_group(account_id).await
    }

    async fn get_members(&self, account_id: &str) -> Result<Vec<TradeGroupMember>> {
        self.get_members(account_id).await
    }
}

#[async_trait]
impl SignalPublisher for ZmqConfigPublisher {
    async fn send_trade_signal(
        &self,
        master_id: &str,
        slave_id: &str,
        signal: &TradeSignal,
    ) -> Result<()> {
        self.send_trade_signal(master_id, slave_id, signal).await
    }
}

pub struct BroadcastNotifier {
    tx: broadcast::Sender<String>,
}

impl BroadcastNotifier {
    pub fn new(tx: broadcast::Sender<String>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl OperationNotifier for BroadcastNotifier {
    async fn notify_received(&self, signal: &TradeSignal) {
        let _ = self.tx.send(format!(
            "trade_received:{}:{}:{}",
            signal.source_account,
            signal.symbol.as_deref().unwrap_or("?"),
            signal.lots.unwrap_or(0.0)
        ));
    }

    async fn notify_copied(&self, slave_id: &str, signal: &TradeSignal, member_id: &str) {
        let _ = self.tx.send(format!(
            "trade_copied:{}:{}:{}:{}",
            slave_id,
            signal.symbol.as_deref().unwrap_or("?"),
            signal.lots.unwrap_or(0.0),
            member_id
        ));
    }
}
