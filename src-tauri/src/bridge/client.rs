use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{client::IntoClientRequest, Message},
    Connector, MaybeTlsStream, WebSocketStream,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::bridge::protocol::*;
use crate::constants::*;
use crate::state::{ConnectionStatus, FigmaStatus, SharedAppState};

/// Force HTTP/1.1 ALPN so CloudFront doesn't negotiate HTTP/2.
/// WebSocket upgrade requires HTTP/1.1 — hop-by-hop headers
/// (Connection, Upgrade) are invalid in HTTP/2.
fn build_tls_connector() -> Option<Connector> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("TLS protocol versions")
    .with_root_certificates(root_store)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Some(Connector::Rustls(std::sync::Arc::new(config)))
}

fn build_ws_url(base_url: &str) -> Result<String, super::BridgeError> {
    let parsed = url::Url::parse(base_url)
        .map_err(|e| super::BridgeError::WebSocket(format!("invalid URL: {e}")))?;
    let ws_scheme = match parsed.scheme() {
        "https" => "wss",
        "http" => "ws",
        s => {
            return Err(super::BridgeError::WebSocket(format!(
                "unsupported scheme: {s}"
            )))
        }
    };
    let host = parsed
        .host_str()
        .ok_or_else(|| super::BridgeError::WebSocket("missing host".into()))?;
    let port_part = parsed.port().map(|p| format!(":{p}")).unwrap_or_default();
    let path = parsed.path().trim_end_matches('/');
    Ok(format!(
        "{ws_scheme}://{host}{port_part}{path}{BRIDGE_WS_PATH}"
    ))
}

fn set_connection_status<R: Runtime>(
    app: &AppHandle<R>,
    state: &SharedAppState,
    status: ConnectionStatus,
) {
    let changed = {
        if let Ok(mut s) = state.lock() {
            if s.connection_status != status {
                s.connection_status = status.clone();
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if changed {
        let _ = app.emit("connection-status-changed", status);
    }
}

pub async fn run_loop<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: AppHandle<R>,
) {
    let mut retry_delay = Duration::from_millis(RECONNECT_BASE_DELAY_MS);

    loop {
        if token.is_cancelled() {
            info!("bridge loop cancelled");
            return;
        }

        let (server_url, jwt, user_id, machine_id) = {
            match state.lock() {
                Ok(s) => (
                    s.server_url.clone(),
                    s.jwt.clone(),
                    s.user_id.clone(),
                    s.machine_id.clone(),
                ),
                Err(_) => {
                    error!("state lock poisoned, stopping bridge loop");
                    return;
                }
            }
        };

        let server_url = match server_url {
            Some(url) => url,
            None => {
                set_connection_status(&app, &state, ConnectionStatus::AuthRequired);
                tokio::select! {
                    _ = token.cancelled() => return,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => continue,
                }
            }
        };

        let is_probe = jwt.is_none();
        let user_id = user_id.unwrap_or_else(|| "probe".to_string());

        let ws_url = match build_ws_url(&server_url) {
            Ok(u) => u,
            Err(e) => {
                error!("invalid server URL: {e}");
                set_connection_status(&app, &state, ConnectionStatus::Reconnecting);
                tokio::select! {
                    _ = token.cancelled() => return,
                    _ = tokio::time::sleep(retry_delay) => {
                        retry_delay = next_delay(retry_delay);
                        continue;
                    }
                }
            }
        };

        set_connection_status(&app, &state, ConnectionStatus::Connecting);
        info!(url = %ws_url, probe = is_probe, "connecting to bridge");

        let mut request = match ws_url.into_client_request() {
            Ok(r) => r,
            Err(e) => {
                error!("failed to build WS request: {e}");
                tokio::select! {
                    _ = token.cancelled() => return,
                    _ = tokio::time::sleep(retry_delay) => {
                        retry_delay = next_delay(retry_delay);
                        continue;
                    }
                }
            }
        };

        if let Some(ref jwt) = jwt {
            match format!("Bearer {jwt}").parse() {
                Ok(val) => {
                    request.headers_mut().insert("Authorization", val);
                }
                Err(e) => {
                    error!("invalid JWT for Authorization header: {e}");
                    set_connection_status(&app, &state, ConnectionStatus::AuthRequired);
                    tokio::select! {
                        _ = token.cancelled() => return,
                        _ = tokio::time::sleep(retry_delay) => {
                            retry_delay = next_delay(retry_delay);
                            continue;
                        }
                    }
                }
            }
        }

        let connector = build_tls_connector();

        let ws_stream = tokio::select! {
            _ = token.cancelled() => return,
            result = connect_async_tls_with_config(request, None, false, connector) => match result {
                Ok((stream, _)) => stream,
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("401") || err_str.contains("403") {
                        warn!("auth rejected: {e}");
                        set_connection_status(&app, &state, ConnectionStatus::AuthRequired);
                    } else {
                        warn!(delay_ms = retry_delay.as_millis(), "connection failed: {e}");
                        set_connection_status(&app, &state, ConnectionStatus::Reconnecting);
                    }
                    tokio::select! {
                        _ = token.cancelled() => return,
                        _ = tokio::time::sleep(retry_delay) => {
                            retry_delay = next_delay(retry_delay);
                            continue;
                        }
                    }
                }
            }
        };

        if is_probe {
            info!("probe connected (waiting for deep link auth)");
            set_connection_status(&app, &state, ConnectionStatus::AuthRequired);
        } else {
            info!("connected to bridge (authenticated)");
            set_connection_status(&app, &state, ConnectionStatus::Connected);
        }
        retry_delay = Duration::from_millis(RECONNECT_BASE_DELAY_MS);

        let figma_reachable = state
            .lock()
            .map(|s| s.figma_status == FigmaStatus::Available)
            .unwrap_or(false);

        if let Err(e) = handle_session(
            ws_stream,
            &token,
            &state,
            &app,
            &user_id,
            &machine_id,
            figma_reachable,
        )
        .await
        {
            warn!("session ended: {e}");
        }

        if token.is_cancelled() {
            return;
        }

        set_connection_status(&app, &state, ConnectionStatus::Reconnecting);
        info!(
            delay_ms = retry_delay.as_millis(),
            "reconnecting after delay"
        );

        tokio::select! {
            _ = token.cancelled() => return,
            _ = tokio::time::sleep(retry_delay) => {
                retry_delay = next_delay(retry_delay);
            }
        }
    }
}

async fn handle_session<R: Runtime>(
    mut ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    token: &CancellationToken,
    state: &SharedAppState,
    app: &AppHandle<R>,
    user_id: &str,
    machine_id: &str,
    figma_reachable: bool,
) -> Result<(), super::BridgeError> {
    let register = BridgeMessage::Register(RegisterMessage {
        user_id: user_id.to_string(),
        machine_id: machine_id.to_string(),
        capabilities: vec![BridgeCapability::FigmaMcp],
        figma_desktop_reachable: figma_reachable,
    });
    send_message(&mut ws, &register).await?;
    info!(figma_reachable, "sent register message");

    let mut heartbeat_interval =
        tokio::time::interval(Duration::from_millis(BRIDGE_HEARTBEAT_INTERVAL_MS));

    let supplementary_hb = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(supplementary_hb);
    let mut supplementary_sent = false;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                let disconnect = BridgeMessage::Disconnect(DisconnectMessage {
                    reason: Some("app shutdown".to_string()),
                });
                let _ = send_message(&mut ws, &disconnect).await;
                return Ok(());
            }

            _ = heartbeat_interval.tick() => {
                let figma = state.lock()
                    .map(|s| s.figma_status == FigmaStatus::Available)
                    .unwrap_or(false);
                let heartbeat = BridgeMessage::Heartbeat(HeartbeatMessage {
                    timestamp: now_ms(),
                    figma_desktop_reachable: Some(figma),
                });
                send_message(&mut ws, &heartbeat).await?;
                if let Ok(mut s) = state.lock() {
                    s.last_heartbeat = Some(std::time::Instant::now());
                }
            }

            _ = &mut supplementary_hb, if !supplementary_sent => {
                supplementary_sent = true;
                let figma = state.lock()
                    .map(|s| s.figma_status == FigmaStatus::Available)
                    .unwrap_or(false);
                let heartbeat = BridgeMessage::Heartbeat(HeartbeatMessage {
                    timestamp: now_ms(),
                    figma_desktop_reachable: Some(figma),
                });
                send_message(&mut ws, &heartbeat).await?;
                info!(figma, "sent supplementary heartbeat (5s after register)");
            }

            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Text(ref text))) if text.len() > BRIDGE_WS_MAX_MESSAGE_BYTES => {
                        warn!(size = text.len(), limit = BRIDGE_WS_MAX_MESSAGE_BYTES, "incoming message exceeds size limit, discarding");
                    }
                    Some(Ok(Message::Text(text))) => {
                        handle_incoming_message(&text, &mut ws, state, app).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("server closed connection");
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        return Err(super::BridgeError::WebSocket(e.to_string()));
                    }
                    None => {
                        return Err(super::BridgeError::WebSocket("stream ended".into()));
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_incoming_message<R: Runtime>(
    text: &str,
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: &SharedAppState,
    app: &AppHandle<R>,
) {
    let msg: BridgeMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!("failed to parse incoming message: {e}");
            return;
        }
    };

    match msg {
        BridgeMessage::McpRequest(req) => {
            info!(
                request_id = %req.request_id,
                tool = %req.tool,
                "received MCP request"
            );

            let start = std::time::Instant::now();
            let response_msg = crate::mcp::proxy::handle_request(&req).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            let response = BridgeMessage::McpResponse(response_msg);
            if let Err(e) = send_message(ws, &response).await {
                error!("failed to send MCP response: {e}");
            }

            if let Ok(mut s) = state.lock() {
                s.mcp_request_count += 1;
                s.last_mcp_request = Some(std::time::Instant::now());
            }

            let _ = app.emit(
                "mcp-request-processed",
                serde_json::json!({
                    "tool": req.tool,
                    "duration_ms": duration_ms
                }),
            );
        }
        BridgeMessage::StatusProbe => {
            info!("received status probe, sending immediate heartbeat");
            let figma = crate::health::figma_check::check_figma_now().await;
            let heartbeat = BridgeMessage::Heartbeat(HeartbeatMessage {
                timestamp: now_ms(),
                figma_desktop_reachable: Some(figma),
            });
            if let Err(e) = send_message(ws, &heartbeat).await {
                error!("failed to send probe heartbeat: {e}");
            }
        }
        _ => {
            warn!("unexpected message type from server");
        }
    }
}

async fn send_message(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    msg: &BridgeMessage,
) -> Result<(), super::BridgeError> {
    let json = serde_json::to_string(msg)?;
    ws.send(Message::Text(json.into()))
        .await
        .map_err(|e| super::BridgeError::WebSocket(e.to_string()))
}

fn next_delay(current: Duration) -> Duration {
    let next = current * 2;
    let max = Duration::from_millis(RECONNECT_MAX_DELAY_MS);
    if next > max {
        max
    } else {
        next
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
