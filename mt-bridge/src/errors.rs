use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("ZMQ Error: {0}")]
    Zmq(#[from] zmq::Error),

    #[error("Serialization Error: {0}")]
    Serde(#[from] rmp_serde::encode::Error),

    #[error("Deserialization Error: {0}")]
    DeSerde(#[from] rmp_serde::decode::Error),

    #[error("Initialization Error: {0}")]
    Init(String),

    #[error("Operation not supported by this strategy")]
    NotSupported,

    #[error("Socket not available")]
    NoSocket,

    #[error("Generic Error: {0}")]
    Generic(String),
}
