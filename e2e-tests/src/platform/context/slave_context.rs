// e2e-tests/src/platform/context/slave.rs
use super::common::{bytes_to_string, EaContextWrapper};
use sankey_copier_zmq::ea_context::EaContext;
use sankey_copier_zmq::ffi::*;
use sankey_copier_zmq::ffi::{
    SGlobalConfig, SPositionInfo, SSlaveConfig, SSymbolMapping, MAX_ACCOUNT_ID_LEN,
};
use sankey_copier_zmq::{
    GlobalConfigMessage, LotCalculationMode, PositionInfo, SlaveConfigMessage, SymbolMapping,
    SyncMode,
};
use std::ops::Deref;

pub struct SlaveContextWrapper {
    base: EaContextWrapper,
}

impl SlaveContextWrapper {
    pub fn new(ctx: *mut EaContext) -> Self {
        Self {
            base: EaContextWrapper::new(ctx),
        }
    }

    pub fn free(self) {
        self.base.free();
    }

    pub fn get_slave_config(&self) -> Option<SlaveConfigMessage> {
        unsafe {
            let mut c_config = SSlaveConfig::default();
            if ea_context_get_slave_config(self.base.raw(), &mut c_config) == 1 {
                let mut config = convert_slave_config(&c_config);

                // Fetch Symbol Mappings
                let count = ea_context_get_symbol_mappings_count(self.base.raw());
                if count > 0 {
                    let mut mappings = vec![SSymbolMapping::default(); count as usize];
                    if ea_context_get_symbol_mappings(self.base.raw(), mappings.as_mut_ptr(), count)
                        == count
                    {
                        config.symbol_mappings =
                            mappings.iter().map(convert_symbol_mapping).collect();
                    }
                }

                Some(config)
            } else {
                None
            }
        }
    }

    pub fn get_global_config(&self) -> Option<GlobalConfigMessage> {
        unsafe {
            let mut c_config = SGlobalConfig::default();
            if ea_context_get_global_config(self.base.raw(), &mut c_config) == 1 {
                Some(convert_global_config(&c_config))
            } else {
                None
            }
        }
    }

    pub fn get_position_snapshot(&self) -> Vec<PositionInfo> {
        unsafe {
            let count = ea_context_get_position_snapshot_count(self.base.raw());
            if count > 0 {
                let mut c_positions = vec![SPositionInfo::default(); count as usize];
                if ea_context_get_position_snapshot(
                    self.base.raw(),
                    c_positions.as_mut_ptr(),
                    count,
                ) == count
                {
                    c_positions.iter().map(convert_position_info).collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        }
    }

    pub fn get_position_snapshot_source_account(&self) -> String {
        unsafe {
            let mut buffer = [0u8; MAX_ACCOUNT_ID_LEN];
            if ea_context_get_position_snapshot_source_account(
                self.base.raw(),
                buffer.as_mut_ptr(),
                MAX_ACCOUNT_ID_LEN as i32,
            ) == 1
            {
                bytes_to_string(&buffer)
            } else {
                String::new()
            }
        }
    }
}

impl Deref for SlaveContextWrapper {
    type Target = EaContextWrapper;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn convert_slave_config(c: &SSlaveConfig) -> SlaveConfigMessage {
    SlaveConfigMessage {
        account_id: bytes_to_string(&c.account_id),
        master_account: bytes_to_string(&c.master_account),
        trade_group_id: bytes_to_string(&c.trade_group_id),
        status: c.status,
        lot_calculation_mode: match c.lot_calculation_mode {
            1 => LotCalculationMode::MarginRatio,
            _ => LotCalculationMode::Multiplier,
        },
        lot_multiplier: Some(c.lot_multiplier),
        reverse_trade: c.reverse_trade != 0,
        symbol_prefix: Some(bytes_to_string(&c.symbol_prefix)).filter(|s| !s.is_empty()),
        symbol_suffix: Some(bytes_to_string(&c.symbol_suffix)).filter(|s| !s.is_empty()),
        symbol_mappings: Vec::new(),
        filters: Default::default(),
        config_version: c.config_version,
        source_lot_min: Some(c.source_lot_min),
        source_lot_max: Some(c.source_lot_max),
        master_equity: Some(c.master_equity),
        sync_mode: match c.sync_mode {
            1 => SyncMode::LimitOrder,
            2 => SyncMode::MarketOrder,
            _ => SyncMode::Skip,
        },
        limit_order_expiry_min: Some(c.limit_order_expiry_min),
        market_sync_max_pips: Some(c.market_sync_max_pips),
        max_slippage: Some(c.max_slippage),
        copy_pending_orders: c.copy_pending_orders != 0,
        // Trade Execution settings
        max_retries: c.max_retries,
        max_signal_delay_ms: c.max_signal_delay_ms,
        use_pending_order_for_delayed: c.use_pending_order_for_delayed != 0,
        allow_new_orders: c.allow_new_orders != 0,
        warning_codes: Vec::new(),
        timestamp: String::new(),
    }
}

fn convert_symbol_mapping(c: &SSymbolMapping) -> SymbolMapping {
    SymbolMapping {
        source_symbol: bytes_to_string(&c.source),
        target_symbol: bytes_to_string(&c.target),
    }
}

fn convert_position_info(c: &SPositionInfo) -> PositionInfo {
    let ot_str = match c.order_type {
        0 => "Buy",
        1 => "Sell",
        2 => "BuyLimit",
        3 => "SellLimit",
        4 => "BuyStop",
        5 => "SellStop",
        _ => "Unknown",
    };

    PositionInfo {
        ticket: c.ticket,
        symbol: bytes_to_string(&c.symbol),
        order_type: ot_str.to_string(),
        lots: c.lots,
        open_price: c.open_price,
        open_time: chrono::DateTime::from_timestamp(c.open_time, 0)
            .unwrap_or_default()
            .to_rfc3339(),
        stop_loss: if c.stop_loss > 0.0 {
            Some(c.stop_loss)
        } else {
            None
        },
        take_profit: if c.take_profit > 0.0 {
            Some(c.take_profit)
        } else {
            None
        },
        magic_number: if c.magic_number != 0 {
            Some(c.magic_number)
        } else {
            None
        },
        comment: Some(bytes_to_string(&c.comment)).filter(|s| !s.is_empty()),
    }
}

fn convert_global_config(c: &SGlobalConfig) -> GlobalConfigMessage {
    GlobalConfigMessage {
        enabled: c.enabled != 0,
        endpoint: bytes_to_string(&c.endpoint),
        batch_size: c.batch_size,
        flush_interval_secs: c.flush_interval_secs,
        log_level: bytes_to_string(&c.log_level),
        timestamp: String::new(),
    }
}
