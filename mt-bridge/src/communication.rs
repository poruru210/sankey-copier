// mt-bridge/src/communication.rs

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

    /// Receive message from Config socket (non-blocking)
    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, String>;

    /// Receive message from Trade socket (non-blocking, Slave only)
    fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, String>;

    /// Subscribe to a topic on Config socket
    fn subscribe_config(&mut self, topic: &str) -> Result<(), String>;
}

// ===========================================================================
// Shared ZMQ Logic
// ===========================================================================

pub struct ZmqResources {
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

    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        match res.sub.recv_into(buffer, zmq::DONTWAIT) {
            Ok(n) => Ok(n as i32),
            Err(zmq::Error::EAGAIN) => Ok(0), // No message available
            Err(e) => Err(e.to_string()),
        }
    }

    fn receive_trade(&mut self, _buffer: &mut [u8]) -> Result<i32, String> {
        Err("Master EA does not receive trade messages".to_string())
    }

    fn subscribe_config(&mut self, topic: &str) -> Result<(), String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(|e| format!("Subscribe failed: {}", e))
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

    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        match res.sub.recv_into(buffer, zmq::DONTWAIT) {
            Ok(n) => Ok(n as i32),
            Err(zmq::Error::EAGAIN) => Ok(0),
            Err(e) => Err(e.to_string()),
        }
    }

    fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, String> {
        // Slave uses same SUB socket for config and trade
        self.receive_config(buffer)
    }

    fn subscribe_config(&mut self, topic: &str) -> Result<(), String> {
        let res = self.resources.as_mut().ok_or("ZMQ not initialized")?;
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(|e| format!("Subscribe failed: {}", e))
    }
}

// ===========================================================================
// Fallback / NoOp Strategy
// ===========================================================================

#[derive(Debug, Default)]
pub struct NoOpStrategy;

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
    fn receive_config(&mut self, _: &mut [u8]) -> Result<i32, String> {
        Ok(0)
    }
    fn receive_trade(&mut self, _: &mut [u8]) -> Result<i32, String> {
        Ok(0)
    }
    fn subscribe_config(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
}
