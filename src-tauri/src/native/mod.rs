#[cfg(windows)]
mod windows;
use crate::models::{AppWindow, Device, Monitor, Session};
#[cfg(windows)]
pub use windows::*;

pub type Routes = [Option<String>; 3];
pub trait AudioRoutingProvider {
    fn list_output_devices(&self) -> Result<Vec<Device>, String>;
    fn list_audio_sessions(&self, devices: &[Device]) -> Result<Vec<Session>, String>;
    fn get_current_route(&self, pid: u32) -> Result<Routes, String>;
    fn set_process_output(&self, pid: u32, endpoint: Option<&str>) -> Result<(), String>;
    fn clear_process_route(&self, pid: u32) -> Result<(), String> {
        self.set_process_output(pid, None)
    }
    fn restore_routes(&self, pid: u32, routes: &Routes) -> Result<(), String>;
    fn supports_process_routing(&self) -> bool;
    fn get_last_error(&self) -> Option<String>;
}
pub trait Platform: AudioRoutingProvider {
    fn icon(&self, _executable: &str) -> Option<String> {
        None
    }
    fn monitors(&self) -> Result<Vec<Monitor>, String>;
    fn windows(&self) -> Result<Vec<AppWindow>, String>;
    fn process(&self, pid: u32) -> Result<(String, u64), String>;
    fn parents(&self) -> std::collections::HashMap<u32, u32>;
    fn test_speaker(&self, endpoint: &str) -> Result<(), String>;
}
