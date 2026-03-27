use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BridgeMessage {
    #[serde(rename = "bridge.register")]
    Register(RegisterMessage),

    #[serde(rename = "bridge.heartbeat")]
    Heartbeat(HeartbeatMessage),

    #[serde(rename = "bridge.disconnect")]
    Disconnect(DisconnectMessage),

    #[serde(rename = "bridge.statusProbe")]
    StatusProbe,

    #[serde(rename = "mcp.request")]
    McpRequest(McpRequestMessage),

    #[serde(rename = "mcp.response")]
    McpResponse(McpResponseMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterMessage {
    pub user_id: String,
    pub machine_id: String,
    pub capabilities: Vec<BridgeCapability>,
    pub figma_desktop_reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatMessage {
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub figma_desktop_reachable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRequestMessage {
    pub request_id: String,
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResponseMessage {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeCapability {
    #[serde(rename = "figma-mcp")]
    FigmaMcp,
}
