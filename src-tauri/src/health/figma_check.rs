use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};
use tokio::time::{interval_at, Instant};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::constants::*;
use crate::state::{FigmaStatus, SharedAppState};

pub async fn check_loop<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: AppHandle<R>,
) {
    let start = Instant::now() + Duration::from_secs(3);
    let mut interval = interval_at(start, Duration::from_millis(FIGMA_HEALTH_CHECK_INTERVAL_MS));

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
                    let _ = app.emit(
                        "figma-status-changed",
                        serde_json::json!({ "available": available }),
                    );
                }
            }
        }
    }
}

async fn check_figma_once() -> FigmaStatus {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

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

    match client.post(FIGMA_MCP_ENDPOINT).json(&request).send().await {
        Ok(_) => FigmaStatus::Available,
        Err(_) => FigmaStatus::Unavailable,
    }
}
