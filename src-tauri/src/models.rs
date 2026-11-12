use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub bounds: Bounds,
    pub primary: bool,
    pub refresh_rate: Option<u32>,
    pub scale: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub pid: u32,
    pub endpoint_id: String,
    pub active: bool,
    pub peak: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct AppWindow {
    pub handle: isize,
    pub pid: u32,
    pub bounds: Bounds,
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RoutingMode {
    Auto,
    Fixed,
    Default,
    Ignore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub executable: String,
    pub mode: RoutingMode,
    pub endpoint_id: Option<String>,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub routing_enabled: bool,
    pub launch_on_startup: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub notifications: bool,
    pub switch_delay_ms: u64,
    pub route_new_apps: bool,
    pub restore_on_exit: bool,
    pub respect_manual: bool,
    pub fallback_endpoint: Option<String>,
    pub reconnect_behavior: String,
    pub debug_logging: bool,
    pub theme: String,
    pub onboarding_completed: bool,
    pub automatic_updates: bool,
    pub update_notifications: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            routing_enabled: false,
            launch_on_startup: false,
            start_minimized: false,
            minimize_to_tray: true,
            notifications: true,
            switch_delay_ms: 350,
            route_new_apps: true,
            restore_on_exit: true,
            respect_manual: true,
            fallback_endpoint: None,
            reconnect_behavior: "wait".into(),
            debug_logging: false,
            theme: "dark".into(),
            onboarding_completed: false,
            automatic_updates: true,
            update_notifications: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub settings: Settings,
    /// Missing key means unassigned; value "default" explicitly follows Windows.
    pub assignments: BTreeMap<String, String>,
    pub rules: Vec<Rule>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            settings: Settings::default(),
            assignments: BTreeMap::new(),
            rules: vec![],
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("This configuration was made by an unsupported AudioFlow version.".into());
        }
        if !(100..=5000).contains(&self.settings.switch_delay_ms) {
            return Err("Switch delay must be between 100 and 5000 ms.".into());
        }
        if !["dark", "light", "system"].contains(&self.settings.theme.as_str()) {
            return Err("Invalid theme.".into());
        }
        if !["wait", "fallback"].contains(&self.settings.reconnect_behavior.as_str()) {
            return Err("Invalid reconnection behavior.".into());
        }
        if self.rules.len() > 500 || self.assignments.len() > 64 {
            return Err("Configuration exceeds the supported size.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.rules {
            if r.id.is_empty()
                || !ids.insert(&r.id)
                || r.executable.trim().is_empty()
                || r.executable.len() > 32768
            {
                return Err(
                    "Every rule needs a unique ID and a nonempty executable name or full path."
                        .into(),
                );
            }
            if r.mode == RoutingMode::Fixed && r.endpoint_id.as_deref().is_none_or(str::is_empty) {
                return Err("Fixed device rules need an output device.".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub icon_data: Option<String>,
    pub pid: u32,
    pub name: String,
    pub executable: String,
    pub monitor_id: Option<String>,
    pub endpoint_ids: Vec<String>,
    pub active: bool,
    pub peak: Option<f32>,
    pub mode: RoutingMode,
    pub status: String,
    pub last_switch: Option<String>,
    pub last_error: Option<String>,
    pub window_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub application: Option<String>,
    pub previous_device: Option<String>,
    pub new_device: Option<String>,
    pub outcome: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub monitors: Vec<Monitor>,
    pub devices: Vec<Device>,
    pub sessions: Vec<Session>,
    pub applications: Vec<Application>,
    pub config: Config,
    pub activity: Vec<Activity>,
    pub routing_supported: bool,
    pub compatibility: String,
    pub successful_changes: u64,
    pub last_scan: Option<String>,
    pub errors: Vec<String>,
    pub version: String,
    pub repository: Option<String>,
    pub updater_configured: bool,
}
