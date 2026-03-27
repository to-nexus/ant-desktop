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
        let msg = match self {
            AppError::Bridge(_) => "connection error".to_string(),
            AppError::Auth(e) => match e {
                crate::auth::AuthError::InvalidJwt(_) => {
                    "invalid authentication token".to_string()
                }
                crate::auth::AuthError::Keychain(_) => "keychain access failed".to_string(),
                crate::auth::AuthError::NoToken => "no authentication token".to_string(),
                _ => "authentication error".to_string(),
            },
            AppError::Mcp(_) => "MCP proxy error".to_string(),
            AppError::Health(_) => "health check error".to_string(),
            AppError::Internal(_) => "internal error".to_string(),
        };
        serializer.serialize_str(&msg)
    }
}
