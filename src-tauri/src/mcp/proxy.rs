use std::time::Duration;

use serde_json::json;
use tokio::sync::Mutex as TokioMutex;
use tracing::{error, info};

use crate::bridge::protocol::{McpRequestMessage, McpResponseMessage};
use crate::constants::*;
use crate::mcp::McpError;

static MCP_MUTEX: TokioMutex<()> = TokioMutex::const_new(());

pub async fn handle_request(req: &McpRequestMessage) -> McpResponseMessage {
    let _guard = MCP_MUTEX.lock().await;

    info!(
        request_id = %req.request_id,
        tool = %req.tool,
        "proxying MCP request to Figma Desktop"
    );

    match proxy_to_figma(req).await {
        Ok(result) => McpResponseMessage {
            request_id: req.request_id.clone(),
            result: Some(result),
            error: None,
        },
        Err(e) => {
            error!(
                request_id = %req.request_id,
                error = %e,
                "MCP proxy failed"
            );
            McpResponseMessage {
                request_id: req.request_id.clone(),
                result: None,
                error: Some(e.to_string()),
            }
        }
    }
}

async fn proxy_to_figma(req: &McpRequestMessage) -> Result<serde_json::Value, McpError> {
    let json_rpc_request = json!({
        "jsonrpc": "2.0",
        "id": req.request_id,
        "method": "tools/call",
        "params": {
            "name": req.tool,
            "arguments": req.args
        }
    });

    let client = reqwest::Client::new();

    let response = tokio::time::timeout(
        Duration::from_millis(BRIDGE_MCP_REQUEST_TIMEOUT_MS),
        client
            .post(FIGMA_MCP_ENDPOINT)
            .json(&json_rpc_request)
            .send(),
    )
    .await
    .map_err(|_| McpError::Timeout)?
    .map_err(|e| {
        if e.is_connect() {
            McpError::FigmaNotRunning
        } else {
            McpError::RequestFailed(e.to_string())
        }
    })?;

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| McpError::RequestFailed(format!("failed to read response: {e}")))?;

    if body_bytes.len() > BRIDGE_WS_MAX_MESSAGE_BYTES {
        return Err(McpError::ResponseTooLarge {
            size: body_bytes.len(),
            limit: BRIDGE_WS_MAX_MESSAGE_BYTES,
        });
    }

    let json_rpc_response: serde_json::Value =
        serde_json::from_slice(&body_bytes)
            .map_err(|e| McpError::RequestFailed(format!("invalid JSON response: {e}")))?;

    if let Some(error) = json_rpc_response.get("error") {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown MCP error");
        return Err(McpError::RequestFailed(msg.to_string()));
    }

    Ok(json_rpc_response
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}
