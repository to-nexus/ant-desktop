use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::state::{AppStateSnapshot, ConnectionStatus, PendingPairing, SharedAppState};
use crate::validation;
use crate::{
    apply_pending_connect, auth, replace_token, save_server_url, save_web_url,
    spawn_baseline_connection, spawn_bridge_task, SharedCancellationToken,
};

#[tauri::command]
pub async fn get_app_state(
    state: tauri::State<'_, SharedAppState>,
) -> Result<AppStateSnapshot, AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
    Ok(AppStateSnapshot::from_state(&guard))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub server_url: Option<String>,
    pub web_url: Option<String>,
    pub has_jwt: bool,
    pub user_id: Option<String>,
    /// Display label for the connected account (address when the token has one).
    pub account: Option<String>,
}

#[tauri::command]
pub async fn get_connection_info(
    state: tauri::State<'_, SharedAppState>,
) -> Result<ConnectionInfo, AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
    Ok(ConnectionInfo {
        server_url: guard.server_url.clone(),
        web_url: guard.web_url.clone(),
        has_jwt: guard.jwt.is_some(),
        user_id: guard.user_id(),
        account: guard.account_label(),
    })
}

#[tauri::command]
pub async fn disconnect<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    shared_token: tauri::State<'_, SharedCancellationToken>,
) -> Result<(), AppError> {
    let new_token = replace_token(shared_token.inner());

    if let Err(e) = auth::keychain::delete_jwt() {
        tracing::warn!("JWT keychain delete failed: {e}");
    }

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.connection_status = ConnectionStatus::AuthRequired;
        s.jwt = None;
        s.account = None;
    }

    let _ = app.emit("connection-status-changed", ConnectionStatus::AuthRequired);

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    spawn_bridge_task(new_token, state.inner().clone(), app);

    Ok(())
}

#[tauri::command]
pub async fn connect<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    shared_token: tauri::State<'_, SharedCancellationToken>,
    server_url: String,
    jwt: String,
    user_id: String,
) -> Result<(), AppError> {
    let server_url = validation::validate_server_url(&server_url)
        .map_err(|e| AppError::Internal(format!("invalid server URL: {e}")))?;

    let new_token = replace_token(shared_token.inner());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    if let Err(e) = auth::keychain::save_jwt(&jwt) {
        tracing::warn!("JWT keychain save failed (proceeding in-memory): {e}");
    }
    save_server_url(&app, &server_url);

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.server_url = Some(server_url);
        s.jwt = Some(jwt);
        s.account = Some(auth::jwt::DesktopClaims {
            sub: user_id,
            email: None,
        });
        s.connection_status = ConnectionStatus::Initial;
    }

    spawn_bridge_task(new_token, state.inner().clone(), app);

    Ok(())
}

/// Apply a deep-link connect that the user explicitly approved. Consumes the
/// parked `pending_connect` and hands it to `apply_pending_connect` — the same
/// path the pairing-matched deep link takes. Errors if nothing is pending or
/// the request sat unanswered past its TTL.
#[tauri::command]
pub async fn confirm_connect<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    shared_token: tauri::State<'_, SharedCancellationToken>,
) -> Result<(), AppError> {
    let pending = {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.pending_connect.take()
    };
    let pending =
        pending.ok_or_else(|| AppError::Internal("no pending connection to confirm".into()))?;

    if pending.is_expired() {
        return Err(AppError::Internal(
            "connect request expired — start the connection again".into(),
        ));
    }

    apply_pending_connect(&app, state.inner(), shared_token.inner(), pending);

    Ok(())
}

/// Discard a parked deep-link connect request the user declined.
///
/// A cold-start deep link is handled *instead of* session restore, so declining
/// one would otherwise leave the app with no bridge at all. Fall back to the
/// baseline connection when nothing is connected.
#[tauri::command]
pub async fn cancel_connect<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
) -> Result<(), AppError> {
    let needs_baseline = {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.pending_connect = None;
        s.jwt.is_none()
    };

    if needs_baseline {
        spawn_baseline_connection(&app);
    }

    Ok(())
}

/// Start a pairing: mint a one-shot nonce, remember it alongside the web host
/// we are about to open, and return the URL to open.
///
/// This is what turns a deep link from "some page asked us to connect" into
/// "the user started this here". The returned URL carries `desktop_pair`; the
/// web app echoes it back as `state` on the deep link, and
/// `process_deep_link_url` redeems it. URL composition lives here so the nonce
/// and the recorded host can never disagree.
#[tauri::command]
pub async fn begin_pairing(
    state: tauri::State<'_, SharedAppState>,
    web_url: String,
) -> Result<String, AppError> {
    let web_url = validation::validate_web_url(&web_url)
        .map_err(|e| AppError::Internal(format!("invalid web URL: {e}")))?;
    if web_url.is_empty() {
        return Err(AppError::Internal("web URL is not configured".into()));
    }

    let mut parsed = url::Url::parse(&web_url)
        .map_err(|e| AppError::Internal(format!("invalid web URL: {e}")))?;
    let web_host = parsed.host_str().map(|h| h.to_ascii_lowercase());

    let nonce = uuid::Uuid::new_v4().to_string();
    parsed.query_pairs_mut().append_pair("desktop_pair", &nonce);

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.pending_pairing = Some(PendingPairing::new(nonce, web_host));
    }

    Ok(parsed.to_string())
}

#[tauri::command]
pub async fn set_web_url<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    url: String,
) -> Result<(), AppError> {
    let url = validation::validate_web_url(&url)
        .map_err(|e| AppError::Internal(format!("invalid web URL: {e}")))?;

    save_web_url(&app, &url);

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.web_url = if url.is_empty() { None } else { Some(url) };
    }

    Ok(())
}

#[tauri::command]
pub async fn set_realtime_base_url<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    shared_token: tauri::State<'_, SharedCancellationToken>,
    url: String,
) -> Result<(), AppError> {
    let url = validation::validate_server_url(&url)
        .map_err(|e| AppError::Internal(format!("invalid server URL: {e}")))?;

    save_server_url(&app, &url);

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.server_url = Some(url);
    }

    let new_token = replace_token(shared_token.inner());
    spawn_bridge_task(new_token, state.inner().clone(), app);

    Ok(())
}
