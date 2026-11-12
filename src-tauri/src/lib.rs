mod commands;
mod config;
mod engine;
mod models;
mod native;
mod tray;
mod updates;

use tauri::Manager;
/// Read-only native probe used for installation diagnostics and hardware validation.
pub fn native_diagnostics() -> Result<serde_json::Value, String> {
    #[cfg(windows)]
    {
        use native::{AudioRoutingProvider, Platform};
        let _apartment = native::ComApartment::new()?;
        let platform = native::WindowsPlatform::new()?;
        let monitors = platform.monitors()?;
        let devices = platform.list_output_devices()?;
        let sessions = platform.list_audio_sessions(&devices)?;
        Ok(
            serde_json::json!({"monitors":monitors,"devices":devices,"audioSessionCount":sessions.len(),"routingSupported":platform.supports_process_routing(),"policyError":platform.get_last_error(),"ownProcessPreference":platform.get_current_route(std::process::id())?}),
        )
    }
    #[cfg(not(windows))]
    Err("AudioFlow requires Windows.".into())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .max_file_size(2_000_000)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            updates::register(app)?;
            let storage = config::Storage::new(&app.path().app_config_dir()?)
                .map_err(std::io::Error::other)?;
            let (config, error) = match storage.load() {
                Ok(c) => (c, None),
                Err(e) => (models::Config::default(), Some(e)),
            };
            log::set_max_level(if config.settings.debug_logging {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            });
            if config.settings.launch_on_startup {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(e) = app.autolaunch().enable() {
                    log::warn!("Windows startup registration could not be refreshed: {e}");
                }
            }
            let start_hidden =
                config.settings.start_minimized && config.settings.onboarding_completed;
            let engine = engine::Engine::start(app.handle().clone(), storage, config, error);
            app.manage(engine);
            tray::create(app)?;
            if !start_hidden {
                if let Some(w) = app.get_webview_window("main") {
                    w.show()?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::scan_now,
            commands::save_settings,
            commands::set_monitor_assignment,
            commands::save_rule,
            commands::delete_rule,
            commands::test_audio_device,
            commands::clear_activity,
            commands::export_config,
            commands::import_config,
            commands::export_activity,
            commands::open_releases,
            commands::prepare_update,
            commands::resume_after_update,
            commands::reset_config,
            commands::export_file
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window
                    .state::<engine::Engine>()
                    .snapshot()
                    .config
                    .settings
                    .minimize_to_tray
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("AudioFlow could not start")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                app.state::<engine::Engine>().shutdown();
            }
        });
}
