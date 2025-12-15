use crate::ea_context::EaContext;
use crate::ffi::helpers::utf16_to_string;

/// Create and initialize an EA Context
///
/// This should be called once in OnInit() and the handle stored in a global variable.
///
/// # Safety
/// All pointers must be valid null-terminated UTF-16 strings
#[no_mangle]
pub unsafe extern "C" fn ea_init(
    account_id: *const u16,
    ea_type: *const u16,
    platform: *const u16,
    account_number: i64,
    broker: *const u16,
    account_name: *const u16,
    server: *const u16,
    currency: *const u16,
    leverage: i64,
) -> *mut EaContext {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let acc_id = match utf16_to_string(account_id) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let et = match utf16_to_string(ea_type) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let plt = match utf16_to_string(platform) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let brk = match utf16_to_string(broker) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let acc_name = match utf16_to_string(account_name) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let srv = match utf16_to_string(server) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let curr = match utf16_to_string(currency) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };

        let context = Box::new(crate::EaContext::new(
            acc_id,
            et,
            plt,
            account_number,
            brk,
            acc_name,
            srv,
            curr,
            leverage,
        ));
        Box::into_raw(context)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

/// Free an EA Context instance
///
/// This should be called in OnDeinit() to clean up the state.
///
/// # Safety
/// - `context` must have been returned by `ea_init()`
/// - `context` must not be null
/// - `context` must only be freed once
#[no_mangle]
pub unsafe extern "C" fn ea_context_free(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            let _ = Box::from_raw(context);
        }
    }));
}

/// Reset the EA state to initial conditions
///
/// This should be called when:
/// - Connection to relay server is lost
/// - EA needs to re-request configuration
///
/// After calling this, `ea_context_should_request_config()` will return true on the next call.
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_reset(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            (*context).reset();
        }
    }));
}

/// Mark that a ConfigMessage has been received
///
/// This should be called when the EA receives a ConfigMessage from the relay server.
/// After calling this, `ea_context_should_request_config()` will return false until
/// `ea_context_reset()` is called.
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_mark_config_requested(context: *mut crate::EaContext) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            (*context).mark_config_requested();
        }
    }));
}

/// Check if the EA should request configuration
///
/// Returns 1 (true) if the EA should request config, 0 (false) otherwise.
///
/// # Safety
/// - `context` must be a valid pointer returned by `ea_init()`
/// - `context` must not have been freed
#[no_mangle]
pub unsafe extern "C" fn ea_context_should_request_config(
    context: *mut crate::EaContext,
    current_trade_allowed: i32,
) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !context.is_null() {
            if (*context).should_request_config(current_trade_allowed != 0) {
                1
            } else {
                0
            }
        } else {
            0
        }
    }));

    result.unwrap_or(0)
}
