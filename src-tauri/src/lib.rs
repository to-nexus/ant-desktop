pub mod auth;
pub mod bridge;
pub mod commands;
pub mod constants;
pub mod error;
pub mod health;
pub mod mcp;
pub mod state;
pub mod tray;

use state::{AppState, SharedAppState};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager, Runtime};
use tauri_plugin_store::StoreExt;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub type SharedCancellationToken = Arc<Mutex<CancellationToken>>;

/// Cancel the current token and replace it with a fresh one.
/// Returns the new token for spawning tasks.
pub fn replace_token(shared: &SharedCancellationToken) -> CancellationToken {
    let mut guard = shared.lock().unwrap();
    guard.cancel();
    let new_token = CancellationToken::new();
    *guard = new_token.clone();
    new_token
}

/// Spawn bridge + health background tasks with the given token.
pub fn spawn_connection_tasks<R: Runtime>(
    token: CancellationToken,
    state: SharedAppState,
    app: tauri::AppHandle<R>,
) {
    tokio::spawn(bridge::client::run_loop(
        token.clone(),
        state.clone(),
        app.clone(),
    ));
    tokio::spawn(health::figma_check::check_loop(token, state, app));
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
        .manage(app_state)
        .manage(cancel_token)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::disconnect,
            commands::connect,
            commands::set_realtime_base_url,
            commands::get_connection_info,
        ])
        .setup(|app| {
            let tray_handle = tray::build_tray(app.handle())?;
            let tray_handle = std::sync::Arc::new(tray_handle);

            {
                let handle = tray_handle.clone();
                let state = app.state::<SharedAppState>().inner().clone();
                app.listen("connection-status-changed", move |_event| {
                    let s = state.lock().unwrap();
                    tray::update_tray(&handle, &s.connection_status, &s.figma_status);
                });
            }
            {
                let handle = tray_handle;
                let state = app.state::<SharedAppState>().inner().clone();
                app.listen("figma-status-changed", move |_event| {
                    let s = state.lock().unwrap();
                    tray::update_tray(&handle, &s.connection_status, &s.figma_status);
                });
            }

            {
                let token = app.state::<SharedCancellationToken>().inner().lock().unwrap().clone();
                let health_state = app.state::<SharedAppState>().inner().clone();
                let health_app = app.handle().clone();
                tokio::spawn(health::figma_check::check_loop(
                    token,
                    health_state,
                    health_app,
                ));
            }

            setup_deep_link(app);
            try_restore_session(app);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_deep_link(app: &tauri::App) {
    let handle = app.handle().clone();
    app.listen("deep-link://new-url", move |event| {
        let payload = event.payload();
        let urls: Vec<String> = match serde_json::from_str(payload) {
            Ok(u) => u,
            Err(_) => vec![payload.to_string()],
        };

        for url_str in urls {
            let url_str = url_str.trim_matches('"');
            info!(url = %url_str, "deep link received");

            if let Ok(params) = auth::deeplink::parse_connect_url(url_str) {
                let user_id = auth::jwt::decode_user_id(&params.token).ok();

                if let Err(e) = auth::keychain::save_jwt(&params.token) {
                    tracing::error!("failed to save JWT: {e}");
                }

                let state = handle.state::<SharedAppState>();
                let shared_token = handle.state::<SharedCancellationToken>();

                let new_token = replace_token(shared_token.inner());

                {
                    let mut s = state.lock().unwrap();
                    s.server_url = Some(params.server.clone());
                    s.jwt = Some(params.token);
                    s.user_id = user_id;
                    s.connection_status = state::ConnectionStatus::Initial;
                }

                save_server_url(&handle, &params.server);

                spawn_connection_tasks(new_token, state.inner().clone(), handle.clone());

                let _ = handle.emit(
                    "auth-received",
                    serde_json::json!({ "server": params.server }),
                );

                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
    });
}

fn try_restore_session(app: &tauri::App) {
    let jwt = match auth::keychain::load_jwt() {
        Ok(Some(jwt)) => jwt,
        _ => return,
    };

    let user_id = match auth::jwt::decode_user_id(&jwt) {
        Ok(uid) => uid,
        Err(_) => return,
    };

    let server_url = load_server_url(app.handle());
    let server_url = match server_url {
        Some(url) => url,
        None => return,
    };

    info!(server = %server_url, "restoring session from keychain/store");

    let state = app.state::<SharedAppState>();
    {
        let mut s = state.lock().unwrap();
        s.server_url = Some(server_url);
        s.jwt = Some(jwt);
        s.user_id = Some(user_id);
    }

    let token = app.state::<SharedCancellationToken>().inner().lock().unwrap().clone();
    let shared_state = state.inner().clone();
    let app_handle = app.handle().clone();

    tokio::spawn(bridge::client::run_loop(token, shared_state, app_handle));
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
