// e2e-tests/src/platform/context/master.rs
use super::common::{bytes_to_string, EaContextWrapper};
use sankey_copier_zmq::ea_context::EaContext;
use sankey_copier_zmq::ffi::*;
use sankey_copier_zmq::ffi::{SMasterConfig, SSyncRequest};
use sankey_copier_zmq::{MasterConfigMessage, SyncRequestMessage};
use std::ops::Deref;

pub struct MasterContextWrapper {
    base: EaContextWrapper,
}

impl MasterContextWrapper {
    pub fn new(ctx: *mut EaContext) -> Self {
        Self {
            base: EaContextWrapper::new(ctx),
        }
    }

    pub fn free(self) {
        self.base.free();
    }

    pub fn get_master_config(&self) -> Option<MasterConfigMessage> {
        unsafe {
            let mut c_config = SMasterConfig::default();
            if ea_context_get_master_config(self.base.raw(), &mut c_config) == 1 {
                Some(convert_master_config(&c_config))
            } else {
                None
            }
        }
    }

    pub fn get_sync_request(&self) -> Option<SyncRequestMessage> {
        unsafe {
            let mut c_req = SSyncRequest::default();
            if ea_context_get_sync_request(self.base.raw(), &mut c_req) == 1 {
                Some(convert_sync_request(&c_req))
            } else {
                None
            }
        }
    }
}

impl Deref for MasterContextWrapper {
    type Target = EaContextWrapper;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn convert_master_config(c: &SMasterConfig) -> MasterConfigMessage {
    MasterConfigMessage {
        account_id: bytes_to_string(&c.account_id),
        status: c.status,
        symbol_prefix: Some(bytes_to_string(&c.symbol_prefix)).filter(|s| !s.is_empty()),
        symbol_suffix: Some(bytes_to_string(&c.symbol_suffix)).filter(|s| !s.is_empty()),
        config_version: c.config_version,
        timestamp: String::new(),
        warning_codes: Vec::new(),
    }
}

fn convert_sync_request(c: &SSyncRequest) -> SyncRequestMessage {
    SyncRequestMessage {
        message_type: "SyncRequest".to_string(),
        slave_account: bytes_to_string(&c.slave_account),
        master_account: bytes_to_string(&c.master_account),
        last_sync_time: Some(bytes_to_string(&c.last_sync_time)).filter(|s| !s.is_empty()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}
