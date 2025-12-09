// mt-bridge/src/ea_context.rs
//
// EA State and Communication Management
//
// Refactored to use Strategy Pattern for Master/Slave communication logic.

use crate::types::{RequestConfigMessage, TradeSignalMessage};
use std::fmt::Debug;
use zmq::{Context, Socket, PUSH, SUB};

// ===========================================================================
// Communication Strategy Interface
// ===========================================================================

pub trait CommunicationStrategy: Send + Debug {
    /// Connect to Relay Server and set up strict subscription rules
    fn connect(&mut self, push_addr: &str, sub_addr: &str, account_id: &str) -> Result<(), String>;

    /// Disconnect and cleanup resources
    fn disconnect(&mut self);

    /// Send data via PUSH socket
    fn send_push(&mut self, data: &[u8]) -> Result<(), String>;

    /// Subscribe to a Master's trade topic (Slave only)
    fn subscribe_trade(&mut self, master_id: &str) -> Result<(), String>;

    /// Get raw pointer to Config SUB socket (for receive loop)
    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String>;

    /// Get raw pointer to Trade SUB socket (Slave only, for receive loop)
    /// Note: In Single-Socket implementation, this may return the same pointer as config socket.
    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String>;
}

// ===========================================================================
// Shared ZMQ Logic
// ===========================================================================

struct ZmqResources {
    _ctx: Context,
    push: Socket,
    sub: Socket,
}

impl Debug for ZmqResources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZmqResources")
            .field("ctx", &"<zmq::Context>")
            .field("push", &"<zmq::Socket>")
            .field("sub", &"<zmq::Socket>")
            .finish()
    }
}

impl ZmqResources {
    fn new(push_addr: &str, sub_addr: &str) -> Result<Self, String> {
        let ctx = Context::new();

        let push = ctx.socket(PUSH).map_err(|e| e.to_string())?;
        push.connect(push_addr)
            .map_err(|e| format!("PUSH connect failed: {}", e))?;

        let sub = ctx.socket(SUB).map_err(|e| e.to_string())?;
        sub.connect(sub_addr)
            .map_err(|e| format!("SUB connect failed: {}", e))?;

        Ok(Self {
            _ctx: ctx,
            push,
            sub,
        })
    }
}

// ===========================================================================
// Master Strategy
// ===========================================================================

#[derive(Debug, Default)]
pub struct MasterStrategy {
    resources: Option<ZmqResources>,
}

impl CommunicationStrategy for MasterStrategy {
    fn connect(&mut self, push_addr: &str, sub_addr: &str, account_id: &str) -> Result<(), String> {
        let res = ZmqResources::new(push_addr, sub_addr)?;

        // Master: Subscribe only to my own config
        // Topic: "config/{account_id}"
        let topic = format!("config/{}", account_id);
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(|e| format!("Subscribe config failed: {}", e))?;

        self.resources = Some(res);
        Ok(())
    }

    fn disconnect(&mut self) {
        self.resources = None;
    }

    fn send_push(&mut self, data: &[u8]) -> Result<(), String> {
        let res = self.resources.as_ref().ok_or("ZMQ not initialized")?;
        res.push.send(data, 0).map_err(|e| e.to_string())
    }

    fn subscribe_trade(&mut self, _master_id: &str) -> Result<(), String> {
        Err("Master EA cannot subscribe to trade topics".to_string())
    }

    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        Ok(res.sub.as_mut_ptr())
    }

    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        Err("Master EA does not have a Trade socket".to_string())
    }
}

// ===========================================================================
// Slave Strategy
// ===========================================================================

#[derive(Debug, Default)]
pub struct SlaveStrategy {
    resources: Option<ZmqResources>,
}

impl CommunicationStrategy for SlaveStrategy {
    fn connect(&mut self, push_addr: &str, sub_addr: &str, account_id: &str) -> Result<(), String> {
        let res = ZmqResources::new(push_addr, sub_addr)?;

        // Slave: Subscribe to my own config
        let topic = format!("config/{}", account_id);
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(|e| format!("Subscribe config failed: {}", e))?;

        self.resources = Some(res);
        Ok(())
    }

    fn disconnect(&mut self) {
        self.resources = None;
    }

    fn send_push(&mut self, data: &[u8]) -> Result<(), String> {
        let res = self.resources.as_ref().ok_or("ZMQ not initialized")?;
        res.push.send(data, 0).map_err(|e| e.to_string())
    }

    fn subscribe_trade(&mut self, master_id: &str) -> Result<(), String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        // Slave: Subscribe to trade signals from a specific master
        // Topic: "trade/{master_id}/"
        let topic = format!("trade/{}/", master_id);
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(|e| format!("Subscribe trade failed: {}", e))?;
        Ok(())
    }

    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        Ok(res.sub.as_mut_ptr())
    }

    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        Ok(res.sub.as_mut_ptr())
    }
}

// ===========================================================================
// Fallback / NoOp Strategy
// ===========================================================================

#[derive(Debug, Default)]
struct NoOpStrategy;

impl CommunicationStrategy for NoOpStrategy {
    fn connect(&mut self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn disconnect(&mut self) {}
    fn send_push(&mut self, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn subscribe_trade(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        Err("NoOp".to_string())
    }
    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        Err("NoOp".to_string())
    }
}

// ===========================================================================
// EaContext (Context)
// ===========================================================================

/// EA Context Manager
/// Holds both static account configuration and dynamic runtime state.
#[derive(Debug)]
pub struct EaContext {
    // --- Static Identity (Set via ea_init) ---
    pub account_id: String,
    pub ea_type: String,  // "Master" or "Slave"
    pub platform: String, // "MT4" or "MT5"
    pub account_number: i64,
    pub broker: String,
    pub account_name: String,
    pub server: String,
    pub currency: String,
    pub leverage: i64,

    // --- Runtime State ---
    /// Config request sent flag
    pub is_config_requested: bool,
    /// Last auto-trading state (for tracking changes)
    pub last_trade_allowed: bool,

    // --- Communication Layer ---
    pub strategy: Box<dyn CommunicationStrategy>,
}

impl EaContext {
    /// Create a new Context with static identity information
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        account_id: String,
        ea_type: String,
        platform: String,
        account_number: i64,
        broker: String,
        account_name: String,
        server: String,
        currency: String,
        leverage: i64,
    ) -> Self {
        // Select strategy based on ea_type
        let strategy: Box<dyn CommunicationStrategy> = match ea_type.as_str() {
            "Master" => Box::new(MasterStrategy::default()),
            "Slave" => Box::new(SlaveStrategy::default()),
            _ => Box::new(NoOpStrategy),
        };

        Self {
            account_id,
            ea_type,
            platform,
            account_number,
            broker,
            account_name,
            server,
            currency,
            leverage,
            is_config_requested: false,
            last_trade_allowed: false,
            strategy,
        }
    }

    // --- Logic Delegation ---

    pub fn connect(&mut self, push_addr: &str, sub_addr: &str) -> Result<(), String> {
        // We pass self.account_id clone if needed, or refs? Strategy expects &str.
        self.strategy.connect(push_addr, sub_addr, &self.account_id)
    }

    pub fn disconnect(&mut self) {
        self.strategy.disconnect();
    }

    pub fn subscribe_trade(&mut self, master_id: &str) -> Result<(), String> {
        self.strategy.subscribe_trade(master_id)
    }

    pub fn send_push(&mut self, data: &[u8]) -> Result<(), String> {
        self.strategy.send_push(data)
    }

    pub fn send_request_config(&mut self, _version: u32) -> Result<(), String> {
        let msg = RequestConfigMessage {
            message_type: "RequestConfig".to_string(),
            account_id: self.account_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            ea_type: self.ea_type.clone(),
        };

        let data =
            rmp_serde::encode::to_vec(&msg).map_err(|e| format!("Serialization failed: {}", e))?;
        self.strategy.send_push(&data)?;

        // Mark as requested to prevent duplicate requests
        self.mark_config_requested();

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_open_signal(
        &mut self,
        ticket: i64,
        symbol: &str,
        order_type: &str,
        lots: f64,
        price: f64,
        sl: f64,
        tp: f64,
        magic: i64,
        comment: &str,
    ) -> Result<(), String> {
        let msg = TradeSignalMessage {
            action: "Open".to_string(),
            ticket,
            symbol: Some(symbol.to_string()),
            order_type: Some(order_type.to_string()),
            lots: Some(lots),
            open_price: Some(price),
            stop_loss: Some(sl),
            take_profit: Some(tp),
            magic_number: Some(magic),
            comment: Some(comment.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source_account: self.account_id.clone(),
            close_ratio: None,
        };

        let data =
            rmp_serde::encode::to_vec(&msg).map_err(|e| format!("Serialization failed: {}", e))?;
        self.strategy.send_push(&data)?;
        Ok(())
    }

    // --- Socket Accessors for FFI (Raw Pointers) ---
    pub fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        self.strategy.get_config_socket_ptr()
    }

    pub fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
        self.strategy.get_trade_socket_ptr()
    }

    // --- Original Logic ---

    /// Determine if RequestConfig should be sent
    pub fn should_request_config(&mut self, current_trade_allowed: bool) -> bool {
        self.last_trade_allowed = current_trade_allowed;
        if self.is_config_requested {
            return false;
        }
        true
    }

    /// Mark that specific config has been requested
    pub fn mark_config_requested(&mut self) {
        self.is_config_requested = true;
    }

    /// Reset state (e.g. on reconnection)
    pub fn reset(&mut self) {
        self.is_config_requested = false;
        // Typically we don't disconnect ZMQ on logic reset, only explicitly.
        // But if we want to ensure clean state, we might want to clear subscriptions?
        // For now, keep connection, just reset state flags.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn create_test_context(ea_type: &str) -> EaContext {
        EaContext::new(
            "test_acc".to_string(),
            ea_type.to_string(),
            "MT5".to_string(),
            123456,
            "TestBroker".to_string(),
            "Test Account".to_string(),
            "TestServer".to_string(),
            "USD".to_string(),
            100,
        )
    }

    #[test]
    fn test_strategy_selection() {
        let master = create_test_context("Master");
        assert_eq!(master.ea_type, "Master");

        let slave = create_test_context("Slave");
        assert_eq!(slave.ea_type, "Slave");
    }

    #[test]
    fn test_initial_state() {
        let ctx = create_test_context("Master");
        assert!(!ctx.is_config_requested);
        assert!(!ctx.last_trade_allowed);
        assert_eq!(ctx.account_id, "test_acc");
        assert_eq!(ctx.broker, "TestBroker");
    }

    #[test]
    fn test_should_request_logic() {
        let mut ctx = create_test_context("Master");
        assert!(ctx.should_request_config(true));
        ctx.mark_config_requested();
        assert!(!ctx.should_request_config(true));
        ctx.reset();
        assert!(ctx.should_request_config(true));
    }

    #[derive(Debug, Default)]
    struct MockStrategy {
        sent_data: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl CommunicationStrategy for MockStrategy {
        fn connect(&mut self, _: &str, _: &str, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn disconnect(&mut self) {}
        fn send_push(&mut self, data: &[u8]) -> Result<(), String> {
            self.sent_data.lock().unwrap().push(data.to_vec());
            Ok(())
        }
        fn subscribe_trade(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
            Err("Mock".to_string())
        }
        fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, String> {
            Err("Mock".to_string())
        }
    }

    #[test]
    fn test_send_request_config() {
        let mut ctx = create_test_context("Slave");
        let sent_data = Arc::new(Mutex::new(Vec::new()));

        ctx.strategy = Box::new(MockStrategy {
            sent_data: sent_data.clone(),
        });

        // This should invoke send_push
        ctx.send_request_config(1)
            .expect("Failed to send request config");

        let data = sent_data.lock().unwrap();
        assert_eq!(data.len(), 1, "Should have sent one message");
    }

    #[test]
    fn test_send_open_signal() {
        let mut ctx = create_test_context("Master");
        let sent_data = Arc::new(Mutex::new(Vec::new()));

        ctx.strategy = Box::new(MockStrategy {
            sent_data: sent_data.clone(),
        });

        ctx.send_open_signal(
            12345,
            "EURUSD",
            "Buy",
            0.1,
            1.1050,
            1.1000,
            1.1100,
            123,
            "Test Comment",
        )
        .expect("Failed to send open signal");

        let data = sent_data.lock().unwrap();
        assert_eq!(data.len(), 1, "Should have sent one message");
    }
}
