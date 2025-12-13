// e2e-tests/src/platform/ea_context_wrapper.rs
//
// Safe Rust Wrapper for EA Context FFI
//
// This wrapper simulates the MQL/C++ client wrapper.
// It uses the new structure-based FFI accessors.

use sankey_copier_zmq::ea_context::EaContext;
use sankey_copier_zmq::ffi::*;
use sankey_copier_zmq::ffi_types::{
    CMasterConfig, CPositionInfo, CSlaveConfig, CSymbolMapping, CSyncRequest, MAX_ACCOUNT_ID_LEN,
};
use sankey_copier_zmq::{
    LotCalculationMode, MasterConfigMessage, PositionInfo, SlaveConfigMessage, SymbolMapping,
    SyncMode, SyncRequestMessage,
};

// Thread-safe wrapper for the raw pointer
// In MQL this would be a class holding the pointer
pub struct EaContextWrapper {
    ctx: *mut EaContext,
}

unsafe impl Send for EaContextWrapper {}
unsafe impl Sync for EaContextWrapper {}

impl EaContextWrapper {
    pub fn new(ctx: *mut EaContext) -> Self {
        Self { ctx }
    }

    pub fn raw(&self) -> *mut EaContext {
        self.ctx
    }

    pub fn free(self) {
        unsafe { ea_context_free(self.ctx) };
    }

    pub fn get_master_config(&self) -> Option<MasterConfigMessage> {
        unsafe {
            let mut c_config = CMasterConfig::default();
            if ea_context_get_master_config(self.ctx, &mut c_config) == 1 {
                Some(convert_master_config(&c_config))
            } else {
                None
            }
        }
    }

    pub fn get_slave_config(&self) -> Option<SlaveConfigMessage> {
        unsafe {
            let mut c_config = CSlaveConfig::default();
            if ea_context_get_slave_config(self.ctx, &mut c_config) == 1 {
                let mut config = convert_slave_config(&c_config);

                // Fetch Symbol Mappings
                let count = ea_context_get_symbol_mappings_count(self.ctx);
                if count > 0 {
                    let mut mappings = vec![CSymbolMapping::default(); count as usize];
                    if ea_context_get_symbol_mappings(self.ctx, mappings.as_mut_ptr(), count)
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

    pub fn get_position_snapshot(&self) -> Vec<PositionInfo> {
        unsafe {
            let count = ea_context_get_position_snapshot_count(self.ctx);
            if count > 0 {
                let mut c_positions = vec![CPositionInfo::default(); count as usize];
                if ea_context_get_position_snapshot(self.ctx, c_positions.as_mut_ptr(), count)
                    == count
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
                self.ctx,
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

    pub fn get_sync_request(&self) -> Option<SyncRequestMessage> {
        unsafe {
            let mut c_req = CSyncRequest::default();
            if ea_context_get_sync_request(self.ctx, &mut c_req) == 1 {
                Some(convert_sync_request(&c_req))
            } else {
                None
            }
        }
    }
}

// --- Conversion Helpers ---

fn bytes_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&x| x == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

fn convert_master_config(c: &CMasterConfig) -> MasterConfigMessage {
    MasterConfigMessage {
        account_id: bytes_to_string(&c.account_id),
        status: c.status,
        symbol_prefix: Some(bytes_to_string(&c.symbol_prefix)).filter(|s| !s.is_empty()),
        symbol_suffix: Some(bytes_to_string(&c.symbol_suffix)).filter(|s| !s.is_empty()),
        config_version: c.config_version,
        timestamp: String::new(), // Not exposed in C struct currently
        warning_codes: Vec::new(),
    }
}

fn convert_slave_config(c: &CSlaveConfig) -> SlaveConfigMessage {
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
        symbol_mappings: Vec::new(), // Populated separately
        filters: Default::default(), // Not exposed yet in flat struct (complex lists)
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
        max_retries: c.max_retries,
        max_signal_delay_ms: c.max_signal_delay_ms,
        use_pending_order_for_delayed: c.use_pending_order_for_delayed != 0,
        allow_new_orders: c.allow_new_orders != 0,
        warning_codes: Vec::new(),
        timestamp: String::new(),
    }
}

fn convert_symbol_mapping(c: &CSymbolMapping) -> SymbolMapping {
    SymbolMapping {
        source_symbol: bytes_to_string(&c.source),
        target_symbol: bytes_to_string(&c.target),
    }
}

fn convert_position_info(c: &CPositionInfo) -> PositionInfo {
    // Map int order_type back to string (reverse of FFI logic)
    // OrderType::Buy = 0, Sell = 1, etc.
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

fn convert_sync_request(c: &CSyncRequest) -> SyncRequestMessage {
    SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        slave_account: bytes_to_string(&c.slave_account),
        master_account: bytes_to_string(&c.master_account),
        last_sync_time: Some(bytes_to_string(&c.last_sync_time)).filter(|s| !s.is_empty()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}
