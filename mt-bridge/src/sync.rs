use crate::ea_context::{EaCommand, EaCommandType};
use crate::ticket_mapper::TicketMapper;
use crate::types::{PositionSnapshotMessage, SlaveConfigMessage, SyncMode};

pub fn process_snapshot(
    snapshot: &PositionSnapshotMessage,
    config: &SlaveConfigMessage,
    mapper: &mut TicketMapper,
    slave_equity: f64,
) -> Vec<EaCommand> {
    let mut commands = Vec::new();

    // Check sync mode
    if config.sync_mode == SyncMode::Skip {
        return commands;
    }

    for pos in &snapshot.positions {
        // Skip if already mapped
        if mapper.get_active(pos.ticket).is_some() {
            continue;
        }

        // Skip if pending but pending order sync disabled?
        // Logic check: PositionSnapshot usually contains Market Positions.
        // Pending orders are synced separately? Or PositionSnapshot can contain pending?
        // Protocol: PositionSnapshot usually contains MARKET positions.
        // If master has pending orders, they might be in a separate snapshot or same list?
        // `PositionInfo` has `order_type`. If "BuyLimit" etc, it is pending.

        let is_pending = is_pending_order(&pos.order_type);
        if is_pending && !config.copy_pending_orders {
            continue;
        }

        // Logic for Pending Sync (if pending exists)
        if is_pending {
            if mapper.get_pending(pos.ticket).is_some() {
                continue;
            }
            // Generate pending order command
            let cmd = create_open_command(pos, config, slave_equity, true);
            if let Some(c) = cmd {
                commands.push(c);
            }
            continue;
        }

        // Market Position Sync
        let mut cmd = match create_open_command(pos, config, slave_equity, false) {
            Some(c) => c,
            None => continue,
        };

        // Adjust for SyncMode (Limit vs Market)
        // For LimitOrder mode, we place a Limit Order at Master's Open Price
        // For MarketOrder mode, we place a Market Order (EaCommandType::Open handles this based on logic in MQL?)
        // Wait, MQL side:
        // `SyncWithLimitOrder` -> Places LIMIT order.
        // `SyncWithMarketOrder` -> Places MARKET order.
        // `EaCommand` doesn't distinguish "Open Market" vs "Open Limit" easily except via `order_type`.
        // If we want to sync with LIMIT order, we must change `order_type` in command to Limit.

        if config.sync_mode == SyncMode::LimitOrder {
            // Change order type to Limit equivalent of the market position
            // e.g. Buy -> BuyLimit
            // But Master's position is "Buy".
            // If we send "BuyLimit" with price = Master's Open Price, MQL will execute OrderSend(OP_BUYLIMIT).
            // Correct.

            let limit_type = convert_to_limit_type(cmd.order_type);
            cmd.order_type = limit_type;

            // Set expiration using param1 (formerly _pad2)
            if let Some(expiry) = config.limit_order_expiry_min {
                if expiry > 0 {
                    cmd.param1 = expiry;
                }
            }
        }

        // For MarketOrder mode, we just send standard Open command.
        // We pass market_sync_max_pips in close_ratio field (reused for Open params)
        // MQL will check deviation if this value > 0.
        if config.sync_mode == SyncMode::MarketOrder {
            if let Some(max_pips) = config.market_sync_max_pips {
                cmd.close_ratio = max_pips;
            }
        }

        commands.push(cmd);
    }

    commands
}

fn is_pending_order(order_type: &str) -> bool {
    matches!(
        order_type,
        "BuyLimit" | "SellLimit" | "BuyStop" | "SellStop"
    )
}

fn convert_to_limit_type(market_type: i32) -> i32 {
    // OrderType::Buy = 0, Sell = 1
    // OrderType::BuyLimit = 2, SellLimit = 3
    match market_type {
        0 => 2,           // Buy -> BuyLimit
        1 => 3,           // Sell -> SellLimit
        _ => market_type, // Already pending or unknown
    }
}

fn create_open_command(
    pos: &crate::types::PositionInfo,
    config: &SlaveConfigMessage,
    slave_equity: f64,
    is_pending: bool,
) -> Option<EaCommand> {
    use crate::constants::OrderType;

    // Parse order type
    let ot = OrderType::try_parse(&pos.order_type)?;

    // Reverse logic
    let final_ot = if config.reverse_trade {
        ot.reverse()
    } else {
        ot
    };

    // Lot calculation (reuse logic from ea_context?)
    // We duplicate or expose helper?
    // Let's replicate simple logic here as we don't want to borrow `EaContext`.
    let lots = transform_lot_size(pos.lots, config, slave_equity);

    let mut cmd = EaCommand::default();
    cmd.command_type = EaCommandType::Open as i32;
    cmd.ticket = pos.ticket;

    copy_string_to_array(&pos.symbol, &mut cmd.symbol);

    cmd.order_type = i32::from(final_ot);
    cmd.volume = lots;
    cmd.price = pos.open_price;
    cmd.sl = pos.stop_loss.unwrap_or(0.0);
    cmd.tp = pos.take_profit.unwrap_or(0.0);
    cmd.magic = pos.magic_number.unwrap_or(0);

    copy_string_to_array(&config.master_account, &mut cmd.source_account);

    // Set timestamp to now? Or keep pos open time?
    // Command timestamp is execution time.
    cmd.timestamp = chrono::Utc::now().timestamp();

    Some(cmd)
}

fn transform_lot_size(lots: f64, config: &SlaveConfigMessage, slave_equity: f64) -> f64 {
    use crate::types::LotCalculationMode;
    match config.lot_calculation_mode {
        LotCalculationMode::Multiplier => lots * config.lot_multiplier.unwrap_or(1.0),
        LotCalculationMode::MarginRatio => {
            if let Some(master_equity) = config.master_equity {
                if master_equity > 0.0 {
                    lots * (slave_equity / master_equity)
                } else {
                    lots
                }
            } else {
                lots
            }
        }
    }
}

fn copy_string_to_array<const N: usize>(s: &str, arr: &mut [u8; N]) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(N - 1);
    arr[..len].copy_from_slice(&bytes[..len]);
    arr[len..].fill(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PositionInfo, SlaveConfigMessage, SyncMode};

    #[test]
    fn test_process_snapshot_empty() {
        let snapshot = PositionSnapshotMessage {
            message_type: "PositionSnapshot".into(),
            source_account: "master".into(),
            positions: vec![],
            timestamp: "2023-01-01T00:00:00Z".into(),
        };
        let config = SlaveConfigMessage::default();
        let mut mapper = TicketMapper::new();

        let cmds = process_snapshot(&snapshot, &config, &mut mapper, 1000.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_process_snapshot_skip_mode() {
        let snapshot = PositionSnapshotMessage {
            message_type: "PositionSnapshot".into(),
            source_account: "master".into(),
            positions: vec![PositionInfo {
                ticket: 1,
                symbol: "EURUSD".into(),
                order_type: "Buy".into(),
                lots: 1.0,
                open_price: 1.0,
                open_time: "".into(),
                stop_loss: None,
                take_profit: None,
                magic_number: None,
                comment: None,
            }],
            timestamp: "".into(),
        };
        let mut config = SlaveConfigMessage::default();
        config.sync_mode = SyncMode::Skip;

        let mut mapper = TicketMapper::new();
        let cmds = process_snapshot(&snapshot, &config, &mut mapper, 1000.0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_process_snapshot_market_mode() {
        let snapshot = PositionSnapshotMessage {
            message_type: "PositionSnapshot".into(),
            source_account: "master".into(),
            positions: vec![PositionInfo {
                ticket: 1,
                symbol: "EURUSD".into(),
                order_type: "Buy".into(),
                lots: 1.0,
                open_price: 1.1000,
                open_time: "".into(),
                stop_loss: None,
                take_profit: None,
                magic_number: None,
                comment: None,
            }],
            timestamp: "".into(),
        };
        let mut config = SlaveConfigMessage::default();
        config.sync_mode = SyncMode::MarketOrder;
        config.master_account = "master".into();

        let mut mapper = TicketMapper::new();
        let cmds = process_snapshot(&snapshot, &config, &mut mapper, 1000.0);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].ticket, 1);
        assert_eq!(cmds[0].order_type, 0); // Buy
        assert_eq!(cmds[0].command_type, EaCommandType::Open as i32);
    }

    #[test]
    fn test_process_snapshot_limit_mode() {
        let snapshot = PositionSnapshotMessage {
            message_type: "PositionSnapshot".into(),
            source_account: "master".into(),
            positions: vec![
                PositionInfo {
                    ticket: 1,
                    symbol: "EURUSD".into(),
                    order_type: "Buy".into(),
                    lots: 1.0,
                    open_price: 1.1000,
                    open_time: "".into(),
                    stop_loss: None,
                    take_profit: None,
                    magic_number: None,
                    comment: None,
                },
                PositionInfo {
                    ticket: 2,
                    symbol: "GBPUSD".into(),
                    order_type: "Sell".into(),
                    lots: 1.0,
                    open_price: 1.2000,
                    open_time: "".into(),
                    stop_loss: None,
                    take_profit: None,
                    magic_number: None,
                    comment: None,
                },
            ],
            timestamp: "".into(),
        };
        let mut config = SlaveConfigMessage::default();
        config.sync_mode = SyncMode::LimitOrder;
        config.master_account = "master".into();

        let mut mapper = TicketMapper::new();
        let cmds = process_snapshot(&snapshot, &config, &mut mapper, 1000.0);

        assert_eq!(cmds.len(), 2);

        // Buy -> BuyLimit (2)
        assert_eq!(cmds[0].ticket, 1);
        assert_eq!(cmds[0].order_type, 2);

        // Sell -> SellLimit (3)
        assert_eq!(cmds[1].ticket, 2);
        assert_eq!(cmds[1].order_type, 3);
    }

    #[test]
    fn test_skip_already_mapped() {
        let snapshot = PositionSnapshotMessage {
            message_type: "PositionSnapshot".into(),
            source_account: "master".into(),
            positions: vec![PositionInfo {
                ticket: 1,
                symbol: "EURUSD".into(),
                order_type: "Buy".into(),
                lots: 1.0,
                open_price: 1.1000,
                open_time: "".into(),
                stop_loss: None,
                take_profit: None,
                magic_number: None,
                comment: None,
            }],
            timestamp: "".into(),
        };
        let mut config = SlaveConfigMessage::default();
        config.sync_mode = SyncMode::MarketOrder;

        let mut mapper = TicketMapper::new();
        mapper.add_active(1, 100); // Already mapped

        let cmds = process_snapshot(&snapshot, &config, &mut mapper, 1000.0);
        assert!(cmds.is_empty());
    }
}
