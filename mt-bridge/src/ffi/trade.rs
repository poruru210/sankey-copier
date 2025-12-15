use crate::ea_context::EaContext;
use crate::ffi::accessors::convert_c_position_to_rust;
use crate::ffi::helpers::utf16_to_string;
use crate::ffi::types::SPositionInfo;

/// Send an Open Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `symbol`, `order_type`, `comment` must be valid null-terminated UTF-16 strings
#[no_mangle]
pub unsafe extern "C" fn ea_send_open_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    symbol: *const u16,
    order_type: *const u16,
    lots: f64,
    price: f64,
    sl: f64,
    tp: f64,
    magic: i64,
    comment: *const u16,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        let sym = match utf16_to_string(symbol) {
            Some(s) => s,
            None => return 0,
        };
        let o_type_str = match utf16_to_string(order_type) {
            Some(s) => s,
            None => return 0,
        };
        let o_type = match crate::constants::OrderType::try_parse(&o_type_str) {
            Some(ot) => ot,
            None => return 0,
        };
        let cmt = match utf16_to_string(comment) {
            Some(s) => s,
            None => return 0,
        };

        match ctx.send_open_signal(ticket, &sym, o_type, lots, price, sl, tp, magic, &cmt) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Send a Close Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
#[no_mangle]
pub unsafe extern "C" fn ea_send_close_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    lots: f64,
    close_ratio: f64,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        match ctx.send_close_signal(ticket, lots, close_ratio) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Send a Modify Trade Signal
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
#[no_mangle]
pub unsafe extern "C" fn ea_send_modify_signal(
    context: *mut crate::EaContext,
    ticket: i64,
    sl: f64,
    tp: f64,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() {
            return 0;
        }
        let ctx = &mut *context;

        match ctx.send_modify_signal(ticket, sl, tp) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}

/// Send a Position Snapshot (Master -> Slave)
///
/// # Safety
/// - context: Valid EaContext pointer
/// - positions: Pointer to array of SPositionInfo
/// - count: Number of positions in the array
#[no_mangle]
pub unsafe extern "C" fn ea_send_position_snapshot(
    context: *mut EaContext,
    positions: *const SPositionInfo,
    count: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if context.is_null() || count < 0 || (count > 0 && positions.is_null()) {
            return 0;
        }
        if count == 0 {
            return 1; // Success, empty snapshot
        }

        let ctx = &mut *context;
        let slice = std::slice::from_raw_parts(positions, count as usize);

        // Convert all positions first (Conversion Layer)
        let rust_positions: Vec<crate::types::PositionInfo> = slice
            .iter()
            .map(|c_pos| convert_c_position_to_rust(c_pos))
            .collect();

        match ctx.send_position_snapshot(rust_positions) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }));

    result.unwrap_or(0)
}
