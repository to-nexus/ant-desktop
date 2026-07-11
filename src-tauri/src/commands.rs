use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::state::{AppStateSnapshot, ConnectionStatus, SharedAppState};
use crate::validation;
use crate::{auth, replace_token, save_server_url, save_web_url, spawn_bridge_task, SharedCancellationToken};

#[tauri::command]
pub async fn get_app_state(state: tauri::State<'_, SharedAppState>) -> Result<AppStateSnapshot, AppError> {
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
        user_id: guard.user_id.clone(),
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
        s.user_id = None;
    }

    let _ = app.emit(
        "connection-status-changed",
        ConnectionStatus::AuthRequired,
    );

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
        s.user_id = Some(user_id);
        s.connection_status = ConnectionStatus::Initial;
    }

    spawn_bridge_task(new_token, state.inner().clone(), app);

    Ok(())
}

/// Apply a deep-link connect that the user explicitly confirmed. Consumes the
/// parked `pending_connect` and performs the same work as `connect` (save JWT,
/// persist server, spawn bridge). No-op error if nothing is pending.
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

    let new_token = replace_token(shared_token.inner());

    if let Err(e) = auth::keychain::save_jwt(&pending.token) {
        tracing::warn!("JWT keychain save failed (proceeding in-memory): {e}");
    }
    save_server_url(&app, &pending.server);

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.server_url = Some(pending.server.clone());
        s.jwt = Some(pending.token);
        s.user_id = pending.user_id;
        s.connection_status = ConnectionStatus::Initial;
    }

    spawn_bridge_task(new_token, state.inner().clone(), app.clone());

    let _ = app.emit("auth-received", serde_json::json!({ "server": pending.server }));

    Ok(())
}

/// Discard a parked deep-link connect request the user declined.
#[tauri::command]
pub async fn cancel_connect(state: tauri::State<'_, SharedAppState>) -> Result<(), AppError> {
    let mut s = state
        .lock()
        .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
    s.pending_connect = None;
    Ok(())
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
