use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Runtime,
};

use crate::state::{ConnectionStatus, FigmaStatus};

const ICON_SIZE: u32 = 22;

fn create_colored_icon(r: u8, g: u8, b: u8) -> Vec<u8> {
    let size = ICON_SIZE as usize;
    let mut rgba = Vec::with_capacity(size * size * 4);
    let center = size as f64 / 2.0;
    let radius = center - 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= radius {
                rgba.extend_from_slice(&[r, g, b, 255]);
            } else if dist <= radius + 1.0 {
                let alpha = ((radius + 1.0 - dist) * 255.0) as u8;
                rgba.extend_from_slice(&[r, g, b, alpha]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    rgba
}

pub fn icon_connected() -> Image<'static> {
    Image::new_owned(create_colored_icon(34, 197, 94), ICON_SIZE, ICON_SIZE)
}

pub fn icon_warning() -> Image<'static> {
    Image::new_owned(create_colored_icon(245, 158, 11), ICON_SIZE, ICON_SIZE)
}

pub fn icon_error() -> Image<'static> {
    Image::new_owned(create_colored_icon(239, 68, 68), ICON_SIZE, ICON_SIZE)
}

pub fn icon_inactive() -> Image<'static> {
    Image::new_owned(create_colored_icon(156, 163, 175), ICON_SIZE, ICON_SIZE)
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
        FigmaStatus::Available => "Figma Desktop: Detected",
        FigmaStatus::Unavailable => "Figma Desktop: Not Running",
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
        .icon(icon_inactive())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("ant-companion")
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
    let icon = match connection {
        ConnectionStatus::Connected => match figma {
            FigmaStatus::Available => icon_connected(),
            _ => icon_warning(),
        },
        ConnectionStatus::Initial | ConnectionStatus::AuthRequired => icon_inactive(),
        _ => icon_error(),
    };
    let _ = handle.tray.set_icon(Some(icon));
    let _ = handle.connection_item.set_text(connection_status_label(connection));
    let _ = handle.figma_item.set_text(figma_status_label(figma));
}
