use std::sync::LazyLock;
use std::time::Duration;

use base64::Engine as _;
use reqwest::Client;
use serde_json::json;
use tokio::sync::Mutex as TokioMutex;
use tracing::{error, info, warn};

use crate::bridge::protocol::{McpRequestMessage, McpResponseMessage};
use crate::constants::*;
use crate::mcp::McpError;

const MCP_ACCEPT: &str = "application/json, text/event-stream";
const ASSET_PROXY_TOOL: &str = "_ant_asset_download";
const FIGMA_LOCAL_ASSET_PREFIXES: &[&str] = &["http://127.0.0.1:3845/", "http://localhost:3845/"];

struct McpSession {
    initialized: bool,
    session_id: Option<String>,
}

static MCP_SESSION: LazyLock<TokioMutex<McpSession>> = LazyLock::new(|| {
    TokioMutex::new(McpSession {
        initialized: false,
        session_id: None,
    })
});

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_millis(BRIDGE_MCP_REQUEST_TIMEOUT_MS))
        .build()
        .expect("failed to build reqwest client")
});

/// Extract JSON from an SSE response body (`event: message\ndata: {...}\n`).
/// Falls back to direct JSON parse when the body is plain JSON.
fn parse_response_body(body: &[u8], is_sse: bool) -> Result<serde_json::Value, McpError> {
    if is_sse {
        let text = std::str::from_utf8(body)
            .map_err(|e| McpError::RequestFailed(format!("invalid UTF-8: {e}")))?;
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                return serde_json::from_str(data).map_err(|e| {
                    McpError::RequestFailed(format!("invalid JSON in SSE data: {e}"))
                });
            }
        }
        Err(McpError::RequestFailed(
            "no data line in SSE response".into(),
        ))
    } else {
        serde_json::from_slice(body)
            .map_err(|e| McpError::RequestFailed(format!("invalid JSON response: {e}")))
    }
}

fn content_type_is_sse(response: &reqwest::Response) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false)
}

/// Reset MCP session state so the next tool call re-initializes.
/// Called when Figma Desktop restarts (detected by health check).
pub async fn reset_session() {
    let mut session = MCP_SESSION.lock().await;
    if session.initialized {
        info!("resetting MCP session (Figma restarted)");
        session.initialized = false;
        session.session_id = None;
    }
}

async fn ensure_initialized(session: &mut McpSession) -> Result<(), McpError> {
    if session.initialized {
        return Ok(());
    }

    info!("initializing MCP session with Figma Desktop");

    let init_body = json!({
        "jsonrpc": "2.0",
        "id": "ant-desktop-init",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "ant-desktop", "version": "0.1.0" }
        }
    });

    let response = HTTP_CLIENT
        .post(FIGMA_MCP_ENDPOINT)
        .header("Accept", MCP_ACCEPT)
        .json(&init_body)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                McpError::FigmaNotRunning
            } else {
                McpError::RequestFailed(format!("initialize failed: {e}"))
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let _ = response.bytes().await;
        return Err(McpError::RequestFailed(format!(
            "initialize returned HTTP {status}"
        )));
    }

    let sid = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let is_sse = content_type_is_sse(&response);
    let body = response
        .bytes()
        .await
        .map_err(|e| McpError::RequestFailed(format!("failed to read init response: {e}")))?;

    let json_rpc = parse_response_body(&body, is_sse)?;
    if let Some(err) = json_rpc.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown initialize error");
        return Err(McpError::RequestFailed(format!(
            "initialize rejected: {msg}"
        )));
    }

    if let Some(ref id) = sid {
        info!(session_id = %id, "captured MCP session ID");
    }

    // Send notifications/initialized
    let mut notif_req = HTTP_CLIENT
        .post(FIGMA_MCP_ENDPOINT)
        .header("Accept", MCP_ACCEPT)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }));

    if let Some(ref id) = sid {
        notif_req = notif_req.header("Mcp-Session-Id", id.as_str());
    }

    match notif_req.send().await {
        Ok(resp) => {
            let _ = resp.bytes().await;
        }
        Err(e) => warn!("notifications/initialized failed (non-fatal): {e}"),
    }

    session.session_id = sid;
    session.initialized = true;
    info!("MCP session initialized");
    Ok(())
}

pub async fn handle_request(req: &McpRequestMessage) -> McpResponseMessage {
    if req.tool == ASSET_PROXY_TOOL {
        info!(
            request_id = %req.request_id,
            "handling asset download proxy"
        );
        return handle_asset_download(req).await;
    }

    let mut session = MCP_SESSION.lock().await;

    info!(
        request_id = %req.request_id,
        tool = %req.tool,
        "proxying MCP request to Figma Desktop"
    );

    match proxy_to_figma(&mut session, req).await {
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
            let error_code = match &e {
                McpError::FigmaNotRunning => "FIGMA_NOT_RUNNING",
                McpError::ResponseTooLarge { .. } => "RESPONSE_TOO_LARGE",
                _ => "MCP_REQUEST_FAILED",
            };
            McpResponseMessage {
                request_id: req.request_id.clone(),
                result: None,
                error: Some(error_code.to_string()),
            }
        }
    }
}

async fn proxy_to_figma(
    session: &mut McpSession,
    req: &McpRequestMessage,
) -> Result<serde_json::Value, McpError> {
    ensure_initialized(session).await?;

    let json_rpc_request = json!({
        "jsonrpc": "2.0",
        "id": req.request_id,
        "method": "tools/call",
        "params": {
            "name": req.tool,
            "arguments": req.args
        }
    });

    let mut request_builder = HTTP_CLIENT
        .post(FIGMA_MCP_ENDPOINT)
        .header("Accept", MCP_ACCEPT)
        .json(&json_rpc_request);

    if let Some(ref sid) = session.session_id {
        request_builder = request_builder.header("Mcp-Session-Id", sid.as_str());
    }

    let response = request_builder.send().await.map_err(|e| {
        if e.is_connect() {
            McpError::FigmaNotRunning
        } else {
            McpError::RequestFailed(e.to_string())
        }
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let _ = response.bytes().await;
        return Err(McpError::RequestFailed(format!(
            "tools/call returned HTTP {status}"
        )));
    }

    let is_sse = content_type_is_sse(&response);

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

    let json_rpc_response = parse_response_body(&body_bytes, is_sse)?;

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

/// Proxy an asset download request from the cloud job worker.
/// The worker cannot reach localhost:3845 directly, so Ant Desktop
/// fetches the asset and returns it as base64.
async fn handle_asset_download(req: &McpRequestMessage) -> McpResponseMessage {
    let url = match req.args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => {
            return McpResponseMessage {
                request_id: req.request_id.clone(),
                result: None,
                error: Some("missing 'url' argument".to_string()),
            };
        }
    };

    let is_local = FIGMA_LOCAL_ASSET_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix));
    if !is_local {
        return McpResponseMessage {
            request_id: req.request_id.clone(),
            result: None,
            error: Some(format!(
                "rejected: URL must start with one of {:?}",
                FIGMA_LOCAL_ASSET_PREFIXES
            )),
        };
    }

    info!(url = %url, "fetching asset from Figma MCP");

    let response = match HTTP_CLIENT.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            let code = if e.is_connect() {
                "FIGMA_NOT_RUNNING"
            } else {
                "ASSET_DOWNLOAD_FAILED"
            };
            error!(url = %url, error = %e, "asset fetch failed");
            return McpResponseMessage {
                request_id: req.request_id.clone(),
                result: None,
                error: Some(code.to_string()),
            };
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let _ = response.bytes().await;
        return McpResponseMessage {
            request_id: req.request_id.clone(),
            result: None,
            error: Some(format!("HTTP {status}")),
        };
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let body = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return McpResponseMessage {
                request_id: req.request_id.clone(),
                result: None,
                error: Some(format!("failed to read body: {e}")),
            };
        }
    };

    if body.len() > BRIDGE_WS_MAX_MESSAGE_BYTES / 2 {
        return McpResponseMessage {
            request_id: req.request_id.clone(),
            result: None,
            error: Some(format!(
                "asset too large: {} bytes (limit {} bytes)",
                body.len(),
                BRIDGE_WS_MAX_MESSAGE_BYTES / 2
            )),
        };
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&body);
    info!(
        url = %url,
        size_bytes = body.len(),
        content_type = %content_type,
        "asset downloaded successfully"
    );

    McpResponseMessage {
        request_id: req.request_id.clone(),
        result: Some(json!({
            "content": [{
                "type": "text",
                "text": b64
            }],
            "mimeType": content_type
        })),
        error: None,
    }
}
