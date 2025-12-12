// mt-bridge/src/communication.rs

use crate::errors::BridgeError;
use std::fmt::Debug;
use zmq::{Context, Socket, PUSH, SUB};

// ===========================================================================
// Communication Strategy Interface
// ===========================================================================

pub trait CommunicationStrategy: Send + Debug {
    /// Connect to Relay Server and set up strict subscription rules
    fn connect(
        &mut self,
        push_addr: &str,
        sub_addr: &str,
        account_id: &str,
    ) -> Result<(), BridgeError>;

    /// Disconnect and cleanup resources
    fn disconnect(&mut self);

    /// Send data via PUSH socket
    fn send_push(&mut self, data: &[u8]) -> Result<(), BridgeError>;

    /// Subscribe to a Master's trade topic (Slave only)
    fn subscribe_trade(&mut self, master_id: &str) -> Result<(), BridgeError>;

    /// Get raw pointer to Config SUB socket (for receive loop)
    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError>;

    /// Get raw pointer to Trade SUB socket (Slave only, for receive loop)
    /// Note: In Single-Socket implementation, this may return the same pointer as config socket.
    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError>;

    /// Receive message from Config socket (non-blocking)
    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError>;

    /// Receive message from Trade socket (non-blocking, Slave only)
    fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError>;

    /// Subscribe to a topic on Config socket
    fn subscribe_config(&mut self, topic: &str) -> Result<(), BridgeError>;
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
    fn new(push_addr: &str, sub_addr: &str) -> Result<Self, BridgeError> {
        let ctx = Context::new();

        let push = ctx.socket(PUSH)?;
        push.connect(push_addr).map_err(BridgeError::Zmq)?;

        let sub = ctx.socket(SUB)?;
        sub.connect(sub_addr).map_err(BridgeError::Zmq)?;

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
    fn connect(
        &mut self,
        push_addr: &str,
        sub_addr: &str,
        account_id: &str,
    ) -> Result<(), BridgeError> {
        let res = ZmqResources::new(push_addr, sub_addr)?;

        // Master: Subscribe only to my own config
        // Topic: "config/{account_id}"
        let topic = format!("config/{}", account_id);
        res.sub.set_subscribe(topic.as_bytes())?;

        self.resources = Some(res);
        Ok(())
    }

    fn disconnect(&mut self) {
        self.resources = None;
    }

    fn send_push(&mut self, data: &[u8]) -> Result<(), BridgeError> {
        let res = self.resources.as_ref().ok_or(BridgeError::NoSocket)?;
        res.push.send(data, 0).map_err(BridgeError::Zmq)
    }

    fn subscribe_trade(&mut self, _master_id: &str) -> Result<(), BridgeError> {
        Err(BridgeError::NotSupported)
    }

    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        Ok(res.sub.as_mut_ptr())
    }

    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        Err(BridgeError::NotSupported)
    }

    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        match res.sub.recv_into(buffer, zmq::DONTWAIT) {
            Ok(n) => Ok(n as i32),
            Err(zmq::Error::EAGAIN) => Ok(0), // No message available
            Err(e) => Err(BridgeError::Zmq(e)),
        }
    }

    fn receive_trade(&mut self, _buffer: &mut [u8]) -> Result<i32, BridgeError> {
        Err(BridgeError::NotSupported)
    }

    fn subscribe_config(&mut self, topic: &str) -> Result<(), BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(BridgeError::Zmq)
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
    fn connect(
        &mut self,
        push_addr: &str,
        sub_addr: &str,
        account_id: &str,
    ) -> Result<(), BridgeError> {
        let res = ZmqResources::new(push_addr, sub_addr)?;

        // Slave: Subscribe to my own config
        let topic = format!("config/{}", account_id);
        res.sub.set_subscribe(topic.as_bytes())?;

        self.resources = Some(res);
        Ok(())
    }

    fn disconnect(&mut self) {
        self.resources = None;
    }

    fn send_push(&mut self, data: &[u8]) -> Result<(), BridgeError> {
        let res = self.resources.as_ref().ok_or(BridgeError::NoSocket)?;
        res.push.send(data, 0).map_err(BridgeError::Zmq)
    }

    fn subscribe_trade(&mut self, master_id: &str) -> Result<(), BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        // Slave: Subscribe to trade signals from a specific master
        // Topic: "trade/{master_id}/"
        let topic = format!("trade/{}/", master_id);
        res.sub.set_subscribe(topic.as_bytes())?;
        Ok(())
    }

    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        Ok(res.sub.as_mut_ptr())
    }

    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        Ok(res.sub.as_mut_ptr())
    }

    fn receive_config(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        match res.sub.recv_into(buffer, zmq::DONTWAIT) {
            Ok(n) => Ok(n as i32),
            Err(zmq::Error::EAGAIN) => Ok(0),
            Err(e) => Err(BridgeError::Zmq(e)),
        }
    }

    fn receive_trade(&mut self, buffer: &mut [u8]) -> Result<i32, BridgeError> {
        // Slave uses same SUB socket for config and trade
        self.receive_config(buffer)
    }

    fn subscribe_config(&mut self, topic: &str) -> Result<(), BridgeError> {
        let res = self.resources.as_mut().ok_or(BridgeError::NoSocket)?;
        res.sub
            .set_subscribe(topic.as_bytes())
            .map_err(BridgeError::Zmq)
    }
}

// ===========================================================================
// Fallback / NoOp Strategy
// ===========================================================================

#[derive(Debug, Default)]
pub struct NoOpStrategy;

impl CommunicationStrategy for NoOpStrategy {
    fn connect(&mut self, _: &str, _: &str, _: &str) -> Result<(), BridgeError> {
        Ok(())
    }
    fn disconnect(&mut self) {}
    fn send_push(&mut self, _: &[u8]) -> Result<(), BridgeError> {
        Ok(())
    }
    fn subscribe_trade(&mut self, _: &str) -> Result<(), BridgeError> {
        Ok(())
    }
    fn get_config_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        Err(BridgeError::NotSupported)
    }
    fn get_trade_socket_ptr(&mut self) -> Result<*mut std::ffi::c_void, BridgeError> {
        Err(BridgeError::NotSupported)
    }
    fn receive_config(&mut self, _: &mut [u8]) -> Result<i32, BridgeError> {
        Ok(0)
    }
    fn receive_trade(&mut self, _: &mut [u8]) -> Result<i32, BridgeError> {
        Ok(0)
    }
    fn subscribe_config(&mut self, _: &str) -> Result<(), BridgeError> {
        Ok(())
    }
}
