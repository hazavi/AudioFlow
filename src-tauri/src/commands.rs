use crate::{
    engine::{Engine, Mutation},
    models::*,
};
use tauri::{AppHandle, Manager};
#[tauri::command]
pub async fn export_file(app: AppHandle, kind: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let snapshot = app.state::<Engine>().snapshot();
        let (name, text) = match kind.as_str() {
            "config" => {
                let path = app
                    .path()
                    .app_config_dir()
                    .map_err(|e| e.to_string())?
                    .join("config.json");
                let text = if path.exists() {
                    std::fs::read_to_string(path).map_err(|e| e.to_string())?
                } else {
                    serde_json::to_string_pretty(&snapshot.config).map_err(|e| e.to_string())?
                };
                ("audioflow-config.json", text)
            }
            "activity" => (
                "audioflow-activity.json",
                serde_json::to_string_pretty(&snapshot.activity).map_err(|e| e.to_string())?,
            ),
            _ => return Err("Unknown export type.".into()),
        };
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name(name)
            .add_filter("JSON", &["json"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = file.into_path().map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("Export could not be saved: {e}"))?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn work(
    app: AppHandle,
    f: impl FnOnce(&Engine) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<Engine>()))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn get_snapshot(engine: tauri::State<'_, Engine>) -> Snapshot {
    engine.snapshot()
}
#[tauri::command]
pub fn scan_now(engine: tauri::State<'_, Engine>) -> Result<(), String> {
    engine.wake()
}
#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    work(app, move |e| e.mutate(Mutation::Settings(settings))).await
}
#[tauri::command]
pub async fn set_monitor_assignment(
    app: AppHandle,
    monitor_id: String,
    endpoint_id: Option<String>,
) -> Result<(), String> {
    work(app, move |e| {
        e.mutate(Mutation::Assignment(monitor_id, endpoint_id))
    })
    .await
}
#[tauri::command]
pub async fn save_rule(app: AppHandle, rule: Rule) -> Result<(), String> {
    work(app, move |e| e.mutate(Mutation::Rule(rule))).await
}
#[tauri::command]
pub async fn delete_rule(app: AppHandle, id: String) -> Result<(), String> {
    work(app, move |e| e.mutate(Mutation::DeleteRule(id))).await
}
#[tauri::command]
pub async fn test_audio_device(app: AppHandle, endpoint_id: String) -> Result<(), String> {
    work(app, move |e| e.test(endpoint_id)).await
}
#[tauri::command]
pub async fn clear_activity(app: AppHandle) -> Result<(), String> {
    work(app, |e| e.mutate(Mutation::ClearActivity)).await
}
#[tauri::command]
pub fn export_config(engine: tauri::State<'_, Engine>) -> Result<String, String> {
    serde_json::to_string_pretty(&engine.snapshot().config).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn import_config(app: AppHandle, json: String) -> Result<(), String> {
    if json.len() > 2_000_000 {
        return Err("Configuration file exceeds 2 MB.".into());
    }
    let config: Config =
        serde_json::from_str(&json).map_err(|e| format!("Invalid configuration: {e}"))?;
    config.validate()?;
    work(app, move |e| e.mutate(Mutation::Import(config))).await
}
#[tauri::command]
pub async fn reset_config(app: AppHandle) -> Result<(), String> {
    work(app, |e| e.mutate(Mutation::Reset)).await
}
#[tauri::command]
pub fn export_activity(engine: tauri::State<'_, Engine>) -> Result<String, String> {
    serde_json::to_string_pretty(&engine.snapshot().activity).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn open_releases(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let repo = crate::updates::repository(&app)
        .ok_or("The GitHub repository has not been configured for this build.")?;
    app.opener()
        .open_url(format!("{repo}/releases"), None::<String>)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn prepare_update(app: AppHandle) -> Result<(), String> {
    work(app, |e| {
        if let Err(error) = e.suspend(true) {
            log::warn!("Stopping audio worker before update: {error}");
        }
        e.shutdown();
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn resume_after_update(app: AppHandle) -> Result<(), String> {
    let storage =
        crate::config::Storage::new(&app.path().app_config_dir().map_err(|e| e.to_string())?)?;
    let handle = app.clone();
    work(app, move |engine| {
        engine.restart(handle, storage);
        Ok(())
    })
    .await
}
