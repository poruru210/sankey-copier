use crate::engine::CopyEngine;
use crate::models::{MasterSettings, SymbolConverter, TradeGroupMember, TradeSignal};
use crate::ports::{OperationNotifier, SignalPublisher, TradeRepository};
use std::sync::Arc;

pub struct TradeCopyService<R: ?Sized, P: ?Sized, N: ?Sized> {
    repository: Arc<R>,
    publisher: Arc<P>,
    notifier: Arc<N>,
    copy_engine: Arc<CopyEngine>,
}

impl<R, P, N> TradeCopyService<R, P, N>
where
    R: TradeRepository + ?Sized,
    P: SignalPublisher + ?Sized,
    N: OperationNotifier + ?Sized,
{
    pub fn new(
        repository: Arc<R>,
        publisher: Arc<P>,
        notifier: Arc<N>,
        copy_engine: Arc<CopyEngine>,
    ) -> Self {
        Self {
            repository,
            publisher,
            notifier,
            copy_engine,
        }
    }

    /// Main processing flow (test target)
    pub async fn process_signal(&self, signal: TradeSignal) {
        tracing::info!("Processing trade signal: {:?}", signal);

        // 1. Notification (Side effect)
        self.notifier.notify_received(&signal).await;

        // 2. DB Data Access (via abstract Repo)
        let master_settings = match self
            .repository
            .get_trade_group(&signal.source_account)
            .await
        {
            Ok(Some(tg)) => tg.master_settings,
            Ok(None) => {
                tracing::warn!(
                    "TradeGroup not found for master {}, using defaults",
                    signal.source_account
                );
                MasterSettings::default()
            }
            Err(e) => {
                tracing::error!("Failed to get TradeGroup: {}", e);
                return;
            }
        };

        let members = match self.repository.get_members(&signal.source_account).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to get members: {}", e);
                return;
            }
        };

        // 3. Loop and Core Logic
        for member in members {
            if !self.copy_engine.should_copy_trade(&signal, &member) {
                tracing::debug!(
                    "Trade filtered out for slave account: {}",
                    member.slave_account
                );
                continue;
            }

            self.execute_copy(&signal, &member, &master_settings).await;
        }
    }

    async fn execute_copy(
        &self,
        signal: &TradeSignal,
        member: &TradeGroupMember,
        settings: &MasterSettings,
    ) {
        let converter = SymbolConverter::from_settings(settings, &member.slave_settings);

        match self
            .copy_engine
            .transform_signal(signal.clone(), member, &converter)
        {
            Ok(transformed) => {
                tracing::info!(
                    "Copying trade to {}: {} {} lots",
                    member.slave_account,
                    transformed.symbol.as_deref().unwrap_or("?"),
                    transformed.lots.unwrap_or(0.0)
                );

                // 4. Publish (via abstract Publisher)
                if let Err(e) = self
                    .publisher
                    .send_trade_signal(&signal.source_account, &member.slave_account, &transformed)
                    .await
                {
                    tracing::error!("Failed to publish: {}", e);
                } else {
                    tracing::debug!(
                        "Sent signal on topic 'trade/{}/{}' for slave '{}'",
                        signal.source_account,
                        member.slave_account,
                        member.slave_account
                    );

                    // 5. Completion Notification
                    self.notifier
                        .notify_copied(&member.slave_account, &transformed, &member.id.to_string())
                        .await;
                }
            }
            Err(e) => tracing::error!("Failed to transform: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SlaveSettings, TradeAction};
    use crate::ports::{MockOperationNotifier, MockSignalPublisher, MockTradeRepository};
    use mockall::predicate::*;

    fn create_dummy_signal(source: &str) -> TradeSignal {
        TradeSignal {
            action: TradeAction::Open,
            source_account: source.to_string(),
            symbol: Some("XAUUSD".to_string()),
            lots: Some(1.0),
            open_price: None,
            ticket: 12345,
            magic_number: None,
            stop_loss: None,
            take_profit: None,
            timestamp: chrono::Utc::now(),
            order_type: None,
            comment: None,
            close_ratio: None,
        }
    }

    fn create_dummy_member(slave: &str) -> TradeGroupMember {
        TradeGroupMember {
            id: 1,
            trade_group_id: "master_1".to_string(),
            slave_account: slave.to_string(),
            slave_settings: SlaveSettings::default(),
            status: 2, // Connected
            warning_codes: vec![],
            enabled_flag: true,
            created_at: "now".to_string(),
            updated_at: "now".to_string(),
        }
    }

    #[tokio::test]
    async fn test_process_signal_successful_copy() {
        let mut mock_repo = MockTradeRepository::new();
        let mut mock_pub = MockSignalPublisher::new();
        let mut mock_notif = MockOperationNotifier::new();

        // Setup Repo behavior
        mock_repo
            .expect_get_trade_group()
            .with(eq("master_1"))
            .returning(|_| Ok(None)); // Use default master settings

        mock_repo
            .expect_get_members()
            .with(eq("master_1"))
            .returning(|_| Ok(vec![create_dummy_member("slave_1")]));

        // Setup Publisher behavior
        mock_pub
            .expect_send_trade_signal()
            .with(
                eq("master_1"),
                eq("slave_1"),
                function(|s: &TradeSignal| s.symbol.as_deref() == Some("XAUUSD")),
            )
            .times(1)
            .returning(|_, _, _| Ok(()));

        // Setup Notifier behavior
        mock_notif
            .expect_notify_received()
            .times(1)
            .return_const(());
        mock_notif
            .expect_notify_copied()
            .with(eq("slave_1"), always(), eq("1"))
            .times(1)
            .return_const(());

        let service = TradeCopyService::new(
            Arc::new(mock_repo),
            Arc::new(mock_pub),
            Arc::new(mock_notif),
            Arc::new(CopyEngine::new()),
        );

        let signal = create_dummy_signal("master_1");
        service.process_signal(signal).await;
    }
}
