use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Client;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::time::{interval_at, Instant};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::constants::*;
use crate::state::{FigmaStatus, SharedAppState};

static HEALTH_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_millis(FIGMA_HEALTH_CHECK_TIMEOUT_MS))
        .build()
        .expect("failed to build health check client")
});

pub async fn check_loop<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: AppHandle<R>,
) {
    let start = Instant::now() + Duration::from_secs(3);
    let mut interval = interval_at(start, Duration::from_millis(FIGMA_HEALTH_CHECK_INTERVAL_MS));

    let mut prev_status = FigmaStatus::Unknown;

    loop {
        tokio::select! {
            _ = token.cancelled() => {
                debug!("figma health check loop cancelled");
                return;
            }
            _ = interval.tick() => {
                let new_status = check_figma_once().await;
                let changed = match state.lock() {
                    Ok(mut s) => {
                        if s.figma_status != new_status {
                            s.figma_status = new_status.clone();
                            true
                        } else {
                            false
                        }
                    }
                    Err(_) => false,
                };
                if changed {
                    let available = matches!(new_status, FigmaStatus::Available);
                    debug!(available, "figma status changed");

                    // Figma came back (restart or first discovery) — reset MCP session
                    // so the next tool call re-initializes the handshake.
                    let was_unavailable = !matches!(prev_status, FigmaStatus::Available);
                    if available && was_unavailable {
                        crate::mcp::proxy::reset_session().await;
                    }

                    let _ = app.emit(
                        "figma-status-changed",
                        serde_json::json!({ "available": available }),
                    );
                }
                prev_status = new_status;
            }
        }
    }
}

async fn check_figma_once() -> FigmaStatus {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "health-check",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "ant-desktop", "version": "0.1.0" }
        }
    });

    match HEALTH_CLIENT.post(FIGMA_MCP_ENDPOINT)
        .header("Accept", "application/json, text/event-stream")
        .json(&request).send().await {
        Ok(_) => FigmaStatus::Available,
        Err(_) => FigmaStatus::Unavailable,
    }
}
