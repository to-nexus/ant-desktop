use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Bridge(#[from] crate::bridge::BridgeError),

    #[error(transparent)]
    Mcp(#[from] crate::mcp::McpError),

    #[error(transparent)]
    Auth(#[from] crate::auth::AuthError),

    #[error(transparent)]
    Health(#[from] crate::health::HealthError),

    #[error("internal: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
