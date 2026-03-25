use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::constants::*;
use crate::state::{FigmaStatus, SharedAppState};

pub async fn check_loop<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: AppHandle<R>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(FIGMA_HEALTH_CHECK_INTERVAL_MS));
    interval.tick().await;

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
        "method": "tools/list",
        "params": {}
    });

    match client.post(FIGMA_MCP_ENDPOINT).json(&request).send().await {
        Ok(resp) if resp.status().is_success() => FigmaStatus::Available,
        Ok(resp) => {
            warn!(status = %resp.status(), "figma MCP returned non-success");
            FigmaStatus::Unavailable
        }
        Err(_) => FigmaStatus::Unavailable,
    }
}
