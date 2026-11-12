use tauri::{App, AppHandle, Emitter};
pub fn repository(app: &AppHandle) -> Option<String> {
    let plugin = app.config().plugins.0.get("updater")?;
    let key = plugin.get("pubkey")?.as_str()?;
    if key.is_empty() {
        return None;
    }
    let endpoint = plugin.get("endpoints")?.as_array()?.first()?.as_str()?;
    let repo = endpoint
        .strip_prefix("https://github.com/")?
        .strip_suffix("/releases/latest/download/latest.json")?;
    if repo.split('/').count() != 2 || repo.contains(['?', '#', ' ']) {
        return None;
    }
    Some(format!("https://github.com/{repo}"))
}
pub fn register(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    if repository(app.handle()).is_some() {
        app.handle()
            .plugin(tauri_plugin_updater::Builder::new().build())?;
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            // Native timer continues while the window is hidden; checks live in the shared frontend service.
            tokio_sleep(std::time::Duration::from_secs(8)).await;
            loop {
                let _ = handle.emit("check-for-updates", ());
                tokio_sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
            }
        });
    }
    Ok(())
}
async fn tokio_sleep(duration: std::time::Duration) {
    // Avoid a new runtime: Tauri's async runtime is backed by Tokio.
    tokio::time::sleep(duration).await;
}
