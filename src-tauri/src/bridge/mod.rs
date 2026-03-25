pub mod client;
pub mod protocol;

pub use protocol::*;

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("not connected")]
    NotConnected,

    #[error("auth required: {0}")]
    AuthRequired(String),
}
