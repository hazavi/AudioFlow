use crate::engine::Engine;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, ExitRequestApi, Manager};

pub const QUIT_ARGUMENT: &str = "--quit-for-install";

#[derive(Default)]
pub struct Shutdown {
    started: AtomicBool,
    complete: AtomicBool,
}

impl Shutdown {
    pub fn request(&self, app: &AppHandle, api: &ExitRequestApi, code: Option<i32>) {
        if self.complete.load(Ordering::Acquire) || code == Some(tauri::RESTART_EXIT_CODE) {
            return;
        }
        api.prevent_exit();
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        // Worker cleanup can update tray controls. Keep the UI loop free to process those calls.
        tauri::async_runtime::spawn_blocking(move || {
            if let Some(engine) = app.try_state::<Engine>() {
                engine.shutdown();
            }
            app.state::<Shutdown>()
                .complete
                .store(true, Ordering::Release);
            app.exit(code.unwrap_or(0));
        });
    }
}
