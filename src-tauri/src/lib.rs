pub mod auth;
pub mod bridge;
pub mod commands;
pub mod constants;
pub mod error;
pub mod health;
pub mod mcp;
pub mod state;
pub mod tray;
pub mod validation;

use state::{AppState, PendingConnect, SharedAppState};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager, Runtime};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_store::StoreExt;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub type SharedCancellationToken = Arc<Mutex<CancellationToken>>;

/// Cancel the current token and replace it with a fresh one.
/// Returns the new token for spawning tasks, or a fresh unmanaged token on lock failure.
pub fn replace_token(shared: &SharedCancellationToken) -> CancellationToken {
    match shared.lock() {
        Ok(mut guard) => {
            guard.cancel();
            let new_token = CancellationToken::new();
            *guard = new_token.clone();
            new_token
        }
        Err(e) => {
            tracing::error!("cancel token lock poisoned: {e}");
            CancellationToken::new()
        }
    }
}

/// Spawn bridge background task with the given token.
/// Health check runs independently (spawned once at startup).
pub fn spawn_bridge_task<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: tauri::AppHandle<R>,
) {
    tauri::async_runtime::spawn(bridge::client::run_loop(token, state, app));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state: SharedAppState = Arc::new(Mutex::new(AppState::new()));
    let cancel_token: SharedCancellationToken = Arc::new(Mutex::new(CancellationToken::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .manage(cancel_token)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::disconnect,
            commands::connect,
            commands::confirm_connect,
            commands::cancel_connect,
            commands::set_realtime_base_url,
            commands::set_web_url,
            commands::get_connection_info,
        ])
        .setup(|app| {
            let tray_handle = tray::build_tray(app.handle())?;
            let tray_handle = std::sync::Arc::new(tray_handle);

            {
                let handle = tray_handle.clone();
                let state = app.state::<SharedAppState>().inner().clone();
                app.listen("connection-status-changed", move |_event| {
                    match state.lock() {
                        Ok(s) => tray::update_tray(&handle, &s.connection_status, &s.figma_status),
                        Err(poisoned) => {
                            let s = poisoned.into_inner();
                            tray::update_tray(&handle, &s.connection_status, &s.figma_status);
                        }
                    }
                });
            }
            {
                let handle = tray_handle;
                let state = app.state::<SharedAppState>().inner().clone();
                app.listen("figma-status-changed", move |_event| match state.lock() {
                    Ok(s) => tray::update_tray(&handle, &s.connection_status, &s.figma_status),
                    Err(poisoned) => {
                        let s = poisoned.into_inner();
                        tray::update_tray(&handle, &s.connection_status, &s.figma_status);
                    }
                });
            }

            // Health check runs independently for the entire app lifetime.
            {
                let health_token = CancellationToken::new();
                let health_state = app.state::<SharedAppState>().inner().clone();
                let health_app = app.handle().clone();
                tauri::async_runtime::spawn(health::figma_check::check_loop(
                    health_token,
                    health_state,
                    health_app,
                ));
            }

            let deep_link_handled = setup_deep_link(app);

            // Load persisted web URL into state
            if let Some(web_url) = load_web_url(app.handle()) {
                if let Ok(mut s) = app.state::<SharedAppState>().lock() {
                    s.web_url = Some(web_url);
                }
            }

            // Keychain + bridge init run off the main thread to avoid
            // blocking the macOS event loop (keychain access can stall).
            // Skip if a deep link was already processed (avoids race condition
            // where session restore cancels or duplicates the deep-link bridge).
            if !deep_link_handled {
                let app_handle = app.handle().clone();
                let state = app.state::<SharedAppState>().inner().clone();
                let shared_token = app.state::<SharedCancellationToken>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if !try_restore_session_async(&app_handle, &state, &shared_token) {
                        try_probe_connection_async(&app_handle, &state, &shared_token);
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}

/// Register deep link handler and process any cold-start URL.
/// Returns `true` if cold-start deep link URLs were found (will be processed
/// off the main thread to avoid keychain blocking the macOS event loop).
fn setup_deep_link(app: &tauri::App) -> bool {
    let handle = app.handle().clone();

    app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        info!(count = urls.len(), "deep link on_open_url fired");
        for url in &urls {
            process_deep_link_url(&handle, url.as_str());
        }
    });

    if let Ok(Some(urls)) = app.deep_link().get_current() {
        if urls.is_empty() {
            return false;
        }
        info!(
            count = urls.len(),
            "deep link get_current found initial URLs"
        );
        let url_strings: Vec<String> = urls.iter().map(|u| u.to_string()).collect();
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            for url_str in &url_strings {
                process_deep_link_url(&handle, url_str);
            }
        });
        return true;
    }

    false
}

fn process_deep_link_url<R: Runtime>(handle: &tauri::AppHandle<R>, url_str: &str) {
    info!(url = %url_str, "deep link received");

    let params = match auth::deeplink::parse_connect_url(url_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(url = %url_str, error = %e, "failed to parse deep link");
            return;
        }
    };

    let user_id = auth::jwt::decode_user_id(&params.token).ok();

    // Do NOT auto-apply. The custom scheme can be triggered by any web page,
    // so silently saving the token + switching servers would be a drive-by
    // account/server swap. Park the request and ask the UI to confirm; the
    // token is saved and the bridge is spawned only in `confirm_connect`.
    let state = handle.state::<SharedAppState>();
    if let Ok(mut s) = state.lock() {
        s.pending_connect = Some(PendingConnect {
            token: params.token.clone(),
            server: params.server.clone(),
            user_id,
        });
    }

    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }

    let _ = handle.emit(
        "auth-connect-request",
        serde_json::json!({ "server": params.server }),
    );
}

/// Try to restore a previous session from keychain. Returns true if restored.
/// Runs off the main thread to avoid blocking the macOS event loop.
fn try_restore_session_async<R: Runtime>(
    app: &tauri::AppHandle<R>,
    state: &SharedAppState,
    shared_token: &SharedCancellationToken,
) -> bool {
    let jwt = match auth::keychain::load_jwt() {
        Ok(Some(jwt)) => jwt,
        _ => return false,
    };

    let user_id = match auth::jwt::decode_user_id(&jwt) {
        Ok(uid) => uid,
        Err(_) => return false,
    };

    let server_url = load_server_url(app);
    let server_url = match server_url {
        Some(url) => url,
        None => return false,
    };

    info!(server = %server_url, "restoring session from keychain/store");

    if let Ok(mut s) = state.lock() {
        s.server_url = Some(server_url);
        s.jwt = Some(jwt);
        s.user_id = Some(user_id);
    }

    let token = match shared_token.lock() {
        Ok(t) => t.clone(),
        Err(_) => return false,
    };

    spawn_bridge_task(token, state.clone(), app.clone());
    true
}

/// Start a probe connection (no JWT) to the default local server.
/// Runs off the main thread to avoid blocking the macOS event loop.
fn try_probe_connection_async<R: Runtime>(
    app: &tauri::AppHandle<R>,
    state: &SharedAppState,
    shared_token: &SharedCancellationToken,
) {
    let server_url = load_server_url(app).unwrap_or_else(|| {
        format!(
            "{}:{}",
            constants::DEFAULT_LOCAL_HOST,
            constants::DEFAULT_LOCAL_PORT
        )
    });

    info!(server = %server_url, "starting probe connection (no JWT)");

    if let Ok(mut s) = state.lock() {
        s.server_url = Some(server_url);
    }

    let token = match shared_token.lock() {
        Ok(t) => t.clone(),
        Err(_) => return,
    };

    spawn_bridge_task(token, state.clone(), app.clone());
}

pub fn save_server_url<R: tauri::Runtime>(app: &tauri::AppHandle<R>, url: &str) {
    if let Ok(store) = app.store("config.json") {
        store.set("realtime_base_url", serde_json::json!(url));
        let _ = store.save();
    }
}

pub fn load_server_url<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<String> {
    let store = app.store("config.json").ok()?;
    store
        .get("realtime_base_url")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

pub fn save_web_url<R: tauri::Runtime>(app: &tauri::AppHandle<R>, url: &str) {
    if let Ok(store) = app.store("config.json") {
        if url.is_empty() {
            store.delete("ant_web_url");
        } else {
            store.set("ant_web_url", serde_json::json!(url));
        }
        let _ = store.save();
    }
}

pub fn load_web_url<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<String> {
    let store = app.store("config.json").ok()?;
    store
        .get("ant_web_url")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}
