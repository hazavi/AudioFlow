use tauri::{App, AppHandle, Emitter};
pub fn repository(app: &AppHandle) -> Option<String> {
    let plugin = app.config().plugins.0.get("updater")?;
    let key = plugin.get("pubkey")?.as_str()?;
    if key.is_empty() {
        return None;
    }
    let endpoint = plugin.get("endpoints")?.as_array()?.first()?.as_str()?;
    repository_url(endpoint)
}
fn repository_url(endpoint: &str) -> Option<String> {
    let repo = endpoint
        .strip_prefix("https://github.com/")?
        .strip_suffix("/releases/latest/download/latest.json")?;
    let (owner, name) = repo.split_once('/')?;
    if owner.is_empty()
        || name.is_empty()
        || matches!(name, "." | "..")
        || !owner
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_links_accept_only_plain_github_repository_paths() {
        assert_eq!(
            repository_url(
                "https://github.com/hazavi/AudioFlow/releases/latest/download/latest.json"
            )
            .as_deref(),
            Some("https://github.com/hazavi/AudioFlow")
        );
        for repo in [
            "/AudioFlow",
            "owner/",
            "owner/..",
            "owner/repo/extra",
            "owner/repo?query",
            "owner/repo#fragment",
            "owner/repo%2fextra",
            "owner/repo\\extra",
            "owner@host/repo",
        ] {
            assert!(repository_url(&format!(
                "https://github.com/{repo}/releases/latest/download/latest.json"
            ))
            .is_none());
        }
        assert!(repository_url(
            "https://github.com.evil.example/hazavi/AudioFlow/releases/latest/download/latest.json"
        )
        .is_none());
    }
}
