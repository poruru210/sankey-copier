use crate::ea_context::EaContext;
use crate::ffi::helpers::copy_string_to_fixed_array;
use crate::ffi::types::{
    SGlobalConfig, SMasterConfig, SPositionInfo, SSlaveConfig, SSymbolMapping, SSyncRequest,
};
use crate::types::{LotCalculationMode, SyncMode};
use chrono::DateTime;

/// Helper to convert raw C byte array (null-terminated or fixed size) to String
/// Used for MQL char arrays (single-byte encoding)
pub(crate) unsafe fn c_byte_array_to_string(ptr: *const u8, max_len: usize) -> String {
    let slice = std::slice::from_raw_parts(ptr, max_len);
    // Find null terminator
    let len = slice.iter().position(|&c| c == 0).unwrap_or(max_len);
    String::from_utf8_lossy(&slice[..len]).into_owned()
}

/// Convert C struct to Rust struct
/// SAFETY: Direct field access is safe because SPositionInfo uses repr(C) with natural alignment
pub(crate) unsafe fn convert_c_position_to_rust(
    c_pos: &SPositionInfo,
) -> crate::types::PositionInfo {
    // Direct field access - safe with natural alignment (repr(C))
    let ticket = c_pos.ticket;
    let lots = c_pos.lots;
    let open_price = c_pos.open_price;
    let open_time = c_pos.open_time;
    let stop_loss = c_pos.stop_loss;
    let take_profit = c_pos.take_profit;
    let magic_number = c_pos.magic_number;
    let order_type = c_pos.order_type;

    let symbol = c_byte_array_to_string(c_pos.symbol.as_ptr(), 32);
    let comment_str = c_byte_array_to_string(c_pos.comment.as_ptr(), 64);

    let order_type_str = match order_type {
        0 => "Buy",
        1 => "Sell",
        2 => "BuyLimit",
        3 => "SellLimit",
        4 => "BuyStop",
        5 => "SellStop",
        _ => "Unknown",
    }
    .to_string();

    let open_time_str = if open_time > 0 {
        match chrono::DateTime::from_timestamp(open_time, 0) {
            Some(dt) => dt.to_rfc3339(),
            None => String::new(),
        }
    } else {
        String::new()
    };

    crate::types::PositionInfo {
        ticket,
        symbol,
        order_type: order_type_str,
        lots,
        open_price,
        open_time: open_time_str,
        stop_loss: if stop_loss.abs() < 1e-6 {
            None
        } else {
            Some(stop_loss)
        },
        take_profit: if take_profit.abs() < 1e-6 {
            None
        } else {
            Some(take_profit)
        },
        magic_number: if magic_number != 0 {
            Some(magic_number)
        } else {
            None
        },
        comment: if comment_str.is_empty() {
            None
        } else {
            Some(comment_str)
        },
    }
}

/// Get the last received Master Config as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - config: Pointer to allocated SMasterConfig struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_master_config(
    context: *const EaContext,
    config: *mut SMasterConfig,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if config.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_master_config {
        let dest = &mut *config;
        copy_string_to_fixed_array(&src.account_id, &mut dest.account_id);
        dest.status = src.status;
        copy_string_to_fixed_array(
            src.symbol_prefix.as_deref().unwrap_or(""),
            &mut dest.symbol_prefix,
        );
        copy_string_to_fixed_array(
            src.symbol_suffix.as_deref().unwrap_or(""),
            &mut dest.symbol_suffix,
        );

        dest.config_version = src.config_version;
        1
    } else {
        0
    }
}

/// Get the last received Slave Config as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - config: Pointer to allocated SSlaveConfig struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_slave_config(
    context: *const EaContext,
    config: *mut SSlaveConfig,
) -> i32 {
    // Mutable access required to pop from queue
    // SAFETY: Single-threaded MQL access assumed.
    let ctx = match (context as *mut EaContext).as_mut() {
        Some(c) => c,
        None => return 0,
    };
    if config.is_null() {
        return 0;
    }

    // Pop next pending config into current slot
    if let Some(next_config) = ctx.pending_slave_configs.pop_front() {
        ctx.current_slave_config = Some(next_config);
    }

    // Read from current slot
    if let Some(src) = &ctx.current_slave_config {
        let dest = &mut *config;
        copy_string_to_fixed_array(&src.account_id, &mut dest.account_id);
        copy_string_to_fixed_array(&src.master_account, &mut dest.master_account);
        copy_string_to_fixed_array(&src.trade_group_id, &mut dest.trade_group_id);

        dest.status = src.status;
        dest.timestamp = src.timestamp; // Already i64
        dest.lot_calculation_mode = match src.lot_calculation_mode {
            LotCalculationMode::Multiplier => 0,
            LotCalculationMode::MarginRatio => 1,
        };
        dest.lot_multiplier = src.lot_multiplier.unwrap_or(0.0);
        dest.reverse_trade = if src.reverse_trade { 1 } else { 0 };

        copy_string_to_fixed_array(
            src.symbol_prefix.as_deref().unwrap_or(""),
            &mut dest.symbol_prefix,
        );
        copy_string_to_fixed_array(
            src.symbol_suffix.as_deref().unwrap_or(""),
            &mut dest.symbol_suffix,
        );

        dest.config_version = src.config_version;
        dest.source_lot_min = src.source_lot_min.unwrap_or(0.0);
        dest.source_lot_max = src.source_lot_max.unwrap_or(0.0);
        dest.master_equity = src.master_equity.unwrap_or(0.0);

        dest.sync_mode = match src.sync_mode {
            SyncMode::Skip => 0,
            SyncMode::LimitOrder => 1,
            SyncMode::MarketOrder => 2,
        };
        dest.limit_order_expiry_min = src.limit_order_expiry_min.unwrap_or(0);
        dest.market_sync_max_pips = src.market_sync_max_pips.unwrap_or(0.0);
        dest.max_slippage = src.max_slippage.unwrap_or(0);
        dest.copy_pending_orders = if src.copy_pending_orders { 1 } else { 0 };

        dest.max_retries = src.max_retries;
        dest.max_signal_delay_ms = src.max_signal_delay_ms;
        dest.use_pending_order_for_delayed = if src.use_pending_order_for_delayed {
            1
        } else {
            0
        };
        dest.allow_new_orders = if src.allow_new_orders { 1 } else { 0 };

        1
    } else {
        0
    }
}

/// Get number of symbol mappings in the last slave config
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_symbol_mappings_count(context: *const EaContext) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if let Some(cfg) = &ctx.current_slave_config {
        cfg.symbol_mappings.len() as i32
    } else {
        0
    }
}

/// Get symbol mappings from the last slave config
///
/// # Safety
/// - mappings: Pointer to array of SSymbolMapping
/// - max_count: Maximum number of mappings to copy
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_symbol_mappings(
    context: *const EaContext,
    mappings: *mut SSymbolMapping,
    max_count: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if mappings.is_null() || max_count <= 0 {
        return 0;
    }

    if let Some(cfg) = &ctx.current_slave_config {
        let count = cfg.symbol_mappings.len().min(max_count as usize);
        let slice = std::slice::from_raw_parts_mut(mappings, count);

        for (i, src) in cfg.symbol_mappings.iter().take(count).enumerate() {
            copy_string_to_fixed_array(&src.source_symbol, &mut slice[i].source);
            copy_string_to_fixed_array(&src.target_symbol, &mut slice[i].target);
        }
        count as i32
    } else {
        0
    }
}

/// Get number of positions in the last position snapshot
///
/// # Safety
/// - context: Valid EaContext pointer
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot_count(context: *const EaContext) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if let Some(snap) = &ctx.last_position_snapshot {
        snap.positions.len() as i32
    } else {
        0
    }
}

/// Get positions from the last position snapshot
///
/// # Safety
/// - positions: Pointer to array of SPositionInfo
/// - max_count: Maximum number of positions to copy
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot(
    context: *const EaContext,
    positions: *mut SPositionInfo,
    max_count: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if positions.is_null() || max_count <= 0 {
        return 0;
    }

    if let Some(snap) = &ctx.last_position_snapshot {
        let count = snap.positions.len().min(max_count as usize);
        let slice = std::slice::from_raw_parts_mut(positions, count);

        for (i, src) in snap.positions.iter().take(count).enumerate() {
            slice[i].ticket = src.ticket;
            copy_string_to_fixed_array(&src.symbol, &mut slice[i].symbol);

            // Map string order type to integer (0=Buy, 1=Sell, etc.)
            let ot = crate::constants::OrderType::try_parse(&src.order_type)
                .unwrap_or(crate::constants::OrderType::Buy); // Default fallback
            slice[i].order_type = i32::from(ot);

            slice[i].lots = src.lots;
            slice[i].open_price = src.open_price;

            // Open Time: String to Timestamp conversion
            if let Ok(dt) = DateTime::parse_from_rfc3339(&src.open_time) {
                slice[i].open_time = dt.timestamp();
            } else {
                slice[i].open_time = 0;
            }

            slice[i].stop_loss = src.stop_loss.unwrap_or(0.0);
            slice[i].take_profit = src.take_profit.unwrap_or(0.0);
            slice[i].magic_number = src.magic_number.unwrap_or(0);

            copy_string_to_fixed_array(src.comment.as_deref().unwrap_or(""), &mut slice[i].comment);
        }
        count as i32
    } else {
        0
    }
}

/// Get the source account from the last position snapshot
///
/// # Safety
/// - context: Valid EaContext pointer
/// - buffer: Pointer to buffer for the source account string
/// - len: Size of buffer (should be at least MAX_ACCOUNT_ID_LEN)
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_position_snapshot_source_account(
    context: *const EaContext,
    buffer: *mut u8,
    len: i32,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if buffer.is_null() || len <= 0 {
        return 0;
    }

    if let Some(snap) = &ctx.last_position_snapshot {
        // Convert to fixed array manually since we have raw pointer
        let slice = std::slice::from_raw_parts_mut(buffer, len as usize);
        let s = &snap.source_account;
        let max_len = (len - 1) as usize;
        let bytes = if s.len() <= max_len {
            s.as_bytes()
        } else {
            let mut end = max_len;
            while end > 0 && !crate::ffi::helpers::is_char_boundary(s, end) {
                end -= 1;
            }
            &s.as_bytes()[..end]
        };
        slice[..bytes.len()].copy_from_slice(bytes);
        slice[bytes.len()..].fill(0);
        1
    } else {
        0
    }
}

/// Get the last received SyncRequest as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - request: Pointer to allocated SSyncRequest struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_sync_request(
    context: *const EaContext,
    request: *mut SSyncRequest,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if request.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_sync_request {
        let dest = &mut *request;
        copy_string_to_fixed_array(&src.slave_account, &mut dest.slave_account);
        copy_string_to_fixed_array(&src.master_account, &mut dest.master_account);
        copy_string_to_fixed_array(
            src.last_sync_time.as_deref().unwrap_or(""),
            &mut dest.last_sync_time,
        );
        1
    } else {
        0
    }
}

/// Get the last received Global Config as a C-compatible struct
///
/// # Safety
/// - context: Valid EaContext pointer
/// - config: Pointer to allocated SGlobalConfig struct
#[no_mangle]
pub unsafe extern "C" fn ea_context_get_global_config(
    context: *const EaContext,
    config: *mut SGlobalConfig,
) -> i32 {
    let ctx = match context.as_ref() {
        Some(c) => c,
        None => return 0,
    };
    if config.is_null() {
        return 0;
    }

    if let Some(src) = &ctx.last_global_config {
        let dest = &mut *config;
        dest.enabled = if src.enabled { 1 } else { 0 };
        copy_string_to_fixed_array(&src.endpoint, &mut dest.endpoint);
        dest.batch_size = src.batch_size;
        dest.flush_interval_secs = src.flush_interval_secs;
        copy_string_to_fixed_array(&src.log_level, &mut dest.log_level);
        copy_string_to_fixed_array(&src.timestamp, &mut dest.timestamp);
        1
    } else {
        0
    }
}
