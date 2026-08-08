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

use state::{AppState, ConnectionStatus, PendingConnect, SharedAppState};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager, Runtime};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_store::StoreExt;
use tokio_util::sync::CancellationToken;
use tracing::info;
use validation::DeepLinkServerPolicy;

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
            commands::begin_pairing,
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

            // Skip if a deep link was already processed (avoids a race where
            // session restore cancels or duplicates the deep-link bridge).
            if !deep_link_handled {
                spawn_baseline_connection(app.handle());
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
        .run(|_app, _event| {
            // `RunEvent::Reopen` is `#[cfg(target_os = "macos")]` in tauri —
            // referencing it unguarded breaks the Linux/Windows builds.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(window) = _app.get_webview_window("main") {
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

/// Apply a connect the user authorized: persist the token + server and restart
/// the bridge against them.
///
/// Single owner for that state transition — `confirm_connect` (explicit
/// approval) and the pairing-matched deep-link path both go through here, so
/// the two can never drift on what "connected" means.
pub fn apply_pending_connect<R: Runtime>(
    app: &tauri::AppHandle<R>,
    state: &SharedAppState,
    shared_token: &SharedCancellationToken,
    pending: PendingConnect,
) {
    let new_token = replace_token(shared_token);

    if let Err(e) = auth::keychain::save_jwt(&pending.token) {
        tracing::warn!("JWT keychain save failed (proceeding in-memory): {e}");
    }
    save_server_url(app, &pending.server);

    if let Ok(mut s) = state.lock() {
        s.server_url = Some(pending.server.clone());
        s.jwt = Some(pending.token);
        s.account = pending.claims;
        s.connection_status = ConnectionStatus::Initial;
        // Any other parked request is moot now — leaving it would pop an
        // approval prompt for a connection that has already been superseded.
        s.pending_connect = None;
    }

    spawn_bridge_task(new_token, state.clone(), app.clone());

    let _ = app.emit(
        "auth-received",
        serde_json::json!({ "server": pending.server }),
    );
}

/// Build the deep-link server allowlist from what the user has actually done:
/// the server they configured, and the web host of a pairing in flight.
fn deeplink_server_policy<R: Runtime>(
    handle: &tauri::AppHandle<R>,
    state: &SharedAppState,
) -> DeepLinkServerPolicy {
    let configured_origin = load_server_url(handle)
        .and_then(|url| url::Url::parse(&url).ok())
        .map(|u| u.origin().ascii_serialization());

    let paired_web_host = state
        .lock()
        .ok()
        .and_then(|s| s.pending_pairing.as_ref().and_then(|p| p.web_host.clone()));

    DeepLinkServerPolicy {
        configured_origin,
        paired_web_host,
    }
}

/// The single trust decision for an inbound `ant-desktop://connect` URI.
///
/// Three outcomes, in order:
///   1. Server origin not allowlisted → refuse, and say so. A silent no-op here
///      would hide both the misconfiguration and the attack.
///   2. `state` matches a live pairing this Desktop started → the user began
///      this flow here, so apply it (consuming the nonce) without a prompt.
///   3. Otherwise → park it and let the UI ask, naming the server *and the
///      account*. The account is the part that matters: an attacker's link can
///      legitimately name the canonical server, so "which server" alone is not
///      a question the user can answer correctly.
fn process_deep_link_url<R: Runtime>(handle: &tauri::AppHandle<R>, url_str: &str) {
    info!(url = %url_str, "deep link received");

    let params = match auth::deeplink::parse_connect_url(url_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(url = %url_str, error = %e, "failed to parse deep link");
            return;
        }
    };

    let state = handle.state::<SharedAppState>().inner().clone();
    let policy = deeplink_server_policy(handle, &state);

    let server = match validation::validate_deeplink_server_url(&params.server_raw, &policy) {
        Ok(s) => s,
        Err(reason) => {
            tracing::error!(server = %params.server_raw, %reason, "rejected deep-link server");
            show_main_window(handle);
            // The rejected string never passed validation, so bound it before it
            // reaches a user-facing notice.
            let _ = handle.emit(
                "auth-connect-rejected",
                serde_json::json!({
                    "server": state::sanitize_display(&params.server_raw),
                    "reason": reason,
                }),
            );
            return;
        }
    };

    let claims = auth::jwt::decode_claims(&params.token).ok();
    let pending = PendingConnect::new(params.token, server.clone(), claims);

    let paired = params
        .state
        .as_deref()
        .is_some_and(|nonce| consume_pairing(&state, nonce));

    if paired {
        info!(server = %server, "deep link matched a local pairing, applying");
        let shared_token = handle.state::<SharedCancellationToken>().inner().clone();
        apply_pending_connect(handle, &state, &shared_token, pending);
        show_main_window(handle);
        return;
    }

    if let Ok(mut s) = state.lock() {
        s.pending_connect = Some(pending);
    }

    show_main_window(handle);

    // The UI reads the parked request out of the state snapshot, so a cold-start
    // deep link survives a webview that was not listening yet. This event is
    // only a "refresh now" nudge.
    let _ = handle.emit(
        "auth-connect-request",
        serde_json::json!({ "server": server }),
    );
}

/// Redeem a pairing nonce. One shot: a match clears the pairing so a replay of
/// the same link lands on the confirmation prompt instead.
fn consume_pairing(state: &SharedAppState, nonce: &str) -> bool {
    let Ok(mut s) = state.lock() else {
        return false;
    };
    match s.pending_pairing.as_ref() {
        Some(p) if p.matches(nonce) => {
            s.pending_pairing = None;
            true
        }
        _ => false,
    }
}

fn show_main_window<R: Runtime>(handle: &tauri::AppHandle<R>) {
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Bring the app to its baseline connection state: restore the stored session,
/// or fall back to an unauthenticated probe.
///
/// Runs off the main thread — keychain access can stall the macOS event loop.
/// Called at startup, and again when the user declines a connect request, so
/// declining returns the app to normal instead of leaving it with no bridge.
pub fn spawn_baseline_connection<R: Runtime>(app: &tauri::AppHandle<R>) {
    let app_handle = app.clone();
    let state = app.state::<SharedAppState>().inner().clone();
    let shared_token = app.state::<SharedCancellationToken>().inner().clone();
    tauri::async_runtime::spawn(async move {
        if !try_restore_session_async(&app_handle, &state, &shared_token) {
            try_probe_connection_async(&app_handle, &state, &shared_token);
        }
    });
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

    let claims = match auth::jwt::decode_claims(&jwt) {
        Ok(c) => c,
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
        s.account = Some(claims);
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
