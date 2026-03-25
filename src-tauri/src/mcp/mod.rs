pub mod proxy;

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Figma Desktop is not running")]
    FigmaNotRunning,

    #[error("MCP request timed out")]
    Timeout,

    #[error("MCP request failed: {0}")]
    RequestFailed(String),

    #[error("response too large: {size} bytes exceeds {limit} byte limit")]
    ResponseTooLarge { size: usize, limit: usize },
}
