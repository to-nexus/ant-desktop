use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Initial,
    Connecting,
    Connected,
    Reconnecting,
    AuthRequired,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FigmaStatus {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Debug)]
pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub figma_status: FigmaStatus,
    pub server_url: Option<String>,
    pub jwt: Option<String>,
    pub user_id: Option<String>,
    pub machine_id: String,
    pub last_heartbeat: Option<Instant>,
    pub mcp_request_count: u64,
    pub last_mcp_request: Option<Instant>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Initial,
            figma_status: FigmaStatus::Unknown,
            server_url: None,
            jwt: None,
            user_id: None,
            machine_id: uuid::Uuid::new_v4().to_string(),
            last_heartbeat: None,
            mcp_request_count: 0,
            last_mcp_request: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateSnapshot {
    pub connection_status: ConnectionStatus,
    pub figma_status: FigmaStatus,
    pub server_url: Option<String>,
    pub machine_id: String,
    pub mcp_request_count: u64,
    pub last_heartbeat_ago_ms: Option<u64>,
    pub last_mcp_request_ago_ms: Option<u64>,
}

impl AppStateSnapshot {
    pub fn from_state(state: &AppState) -> Self {
        let now = Instant::now();
        Self {
            connection_status: state.connection_status.clone(),
            figma_status: state.figma_status.clone(),
            server_url: state.server_url.clone(),
            machine_id: state.machine_id.clone(),
            mcp_request_count: state.mcp_request_count,
            last_heartbeat_ago_ms: state.last_heartbeat.map(|t| now.duration_since(t).as_millis() as u64),
            last_mcp_request_ago_ms: state.last_mcp_request.map(|t| now.duration_since(t).as_millis() as u64),
        }
    }
}
