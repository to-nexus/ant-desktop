use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Runtime,
};

use crate::state::{ConnectionStatus, FigmaStatus};

static ICON_DEFAULT: &[u8] = include_bytes!("../../icons/32x32.png");

fn default_icon() -> Image<'static> {
    Image::from_bytes(ICON_DEFAULT).expect("embedded icon must be valid")
}

pub struct TrayHandle<R: Runtime> {
    pub tray: TrayIcon<R>,
    pub connection_item: MenuItem<R>,
    pub figma_item: MenuItem<R>,
}

fn connection_status_label(status: &ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Connected => "Ant Cloud: Connected",
        ConnectionStatus::Connecting => "Ant Cloud: Connecting...",
        ConnectionStatus::Reconnecting => "Ant Cloud: Reconnecting...",
        ConnectionStatus::AuthRequired => "Ant Cloud: Auth Required",
        ConnectionStatus::Disconnected => "Ant Cloud: Disconnected",
        ConnectionStatus::Initial => "Ant Cloud: Not Connected",
    }
}

fn figma_status_label(status: &FigmaStatus) -> &'static str {
    match status {
        FigmaStatus::Available => "Figma Desktop: Connected",
        FigmaStatus::Unavailable => "Figma Desktop: Not Connected",
        FigmaStatus::Unknown => "Figma Desktop: Unknown",
    }
}

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayHandle<R>> {
    let connection_item =
        MenuItem::with_id(app, "conn_status", "Ant Cloud: Not Connected", false, None::<&str>)?;
    let figma_item =
        MenuItem::with_id(app, "figma_status", "Figma Desktop: Unknown", false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let logs_item = MenuItem::with_id(app, "logs", "View Logs...", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &connection_item,
            &figma_item,
            &sep1,
            &settings_item,
            &logs_item,
            &sep2,
            &quit_item,
        ],
    )?;

    let tray = TrayIconBuilder::new()
        .icon(default_icon())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Ant Desktop")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "settings" | "logs" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(TrayHandle {
        tray,
        connection_item,
        figma_item,
    })
}

pub fn update_tray<R: Runtime>(
    handle: &TrayHandle<R>,
    connection: &ConnectionStatus,
    figma: &FigmaStatus,
) {
    let _ = handle.connection_item.set_text(connection_status_label(connection));
    let _ = handle.figma_item.set_text(figma_status_label(figma));
}
