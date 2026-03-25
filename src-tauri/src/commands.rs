use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::state::{AppStateSnapshot, ConnectionStatus, SharedAppState};
use crate::{auth, replace_token, save_server_url, spawn_connection_tasks, SharedCancellationToken};

#[tauri::command]
pub fn get_app_state(state: tauri::State<SharedAppState>) -> Result<AppStateSnapshot, AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
    Ok(AppStateSnapshot::from_state(&guard))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub server_url: Option<String>,
    pub has_jwt: bool,
    pub user_id: Option<String>,
}

#[tauri::command]
pub fn get_connection_info(
    state: tauri::State<SharedAppState>,
) -> Result<ConnectionInfo, AppError> {
    let guard = state
        .lock()
        .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
    Ok(ConnectionInfo {
        server_url: guard.server_url.clone(),
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
    {
        let guard = shared_token.lock().unwrap();
        guard.cancel();
    }

    let _ = auth::keychain::delete_jwt();

    {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.connection_status = ConnectionStatus::Disconnected;
        s.jwt = None;
        s.user_id = None;
    }

    let _ = app.emit(
        "connection-status-changed",
        ConnectionStatus::Disconnected,
    );

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
    let new_token = replace_token(shared_token.inner());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let _ = auth::keychain::save_jwt(&jwt);
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

    spawn_connection_tasks(new_token, state.inner().clone(), app);

    Ok(())
}

#[tauri::command]
pub async fn set_realtime_base_url<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, SharedAppState>,
    shared_token: tauri::State<'_, SharedCancellationToken>,
    url: String,
) -> Result<(), AppError> {
    save_server_url(&app, &url);

    let has_jwt = {
        let mut s = state
            .lock()
            .map_err(|e| AppError::Internal(format!("state lock poisoned: {e}")))?;
        s.server_url = Some(url);
        s.jwt.is_some()
    };

    if has_jwt {
        let new_token = replace_token(shared_token.inner());
        spawn_connection_tasks(new_token, state.inner().clone(), app);
    }

    Ok(())
}
