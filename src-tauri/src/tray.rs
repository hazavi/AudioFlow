use crate::engine::{Engine, Mutation};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
pub fn create(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open AudioFlow", true, None::<&str>)?;
    let toggle = CheckMenuItem::with_id(
        app,
        "toggle",
        "Automatic Routing",
        true,
        app.state::<Engine>()
            .snapshot()
            .config
            .settings
            .routing_enabled,
        None::<&str>,
    )?;
    let scan = MenuItem::with_id(app, "scan", "Scan Now", true, None::<&str>)?;
    let monitors = MenuItem::with_id(app, "monitors", "Connected Monitors", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Open Settings", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit AudioFlow", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &scan, &monitors, &settings, &exit])?;
    let tray = TrayIconBuilder::with_id("audioflow")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/32x32.png"
        ))?)
        .tooltip("AudioFlow · Routing paused")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "exit" => {
                app.exit(0);
            }
            "scan" => {
                let _ = app.state::<Engine>().wake();
            }
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let engine = app.state::<Engine>();
                    let mut settings = engine.snapshot().config.settings;
                    settings.routing_enabled = !settings.routing_enabled;
                    if let Err(e) = engine.mutate(Mutation::Settings(settings)) {
                        log::error!("Tray toggle failed: {e}");
                    }
                });
            }
            page => {
                let page = match page {
                    "monitors" => "Monitors",
                    "settings" => "Settings",
                    _ => "Dashboard",
                };
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app.emit("navigate", page);
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = tray.app_handle().emit("navigate", "Dashboard");
                }
            }
        })
        .build(app)?;
    use tauri::Listener;
    let app_handle = app.handle().clone();
    app.listen("state-changed", move |_| {
        let snapshot = app_handle.state::<Engine>().snapshot();
        let enabled = snapshot.config.settings.routing_enabled;
        let _ = toggle.set_checked(enabled);
        let _ = tray.set_tooltip(Some(if !snapshot.routing_supported {
            "AudioFlow · Routing unavailable on this Windows build"
        } else if enabled {
            "AudioFlow · Automatic routing active"
        } else {
            "AudioFlow · Routing paused"
        }));
    });
    Ok(())
}
