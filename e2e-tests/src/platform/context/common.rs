// e2e-tests/src/platform/context/common.rs
use sankey_copier_zmq::ea_context::EaContext;
use sankey_copier_zmq::ffi::*;

// Thread-safe wrapper for the raw pointer
pub struct EaContextWrapper {
    pub(crate) ctx: *mut EaContext,
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
}

// Helper: Convert null-terminated byte slice to String
pub fn bytes_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&x| x == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}
