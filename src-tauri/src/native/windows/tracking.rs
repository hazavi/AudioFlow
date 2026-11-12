use super::*;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::Sender,
        Mutex, OnceLock,
    },
    thread,
};
use windows::Win32::UI::Accessibility::*;
static WAKE: OnceLock<Mutex<Option<Sender<crate::engine::Message>>>> = OnceLock::new();
unsafe extern "system" fn event(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if object != 0 {
        return;
    }
    if let Some(lock) = WAKE.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(crate::engine::Message::Wake);
            }
        }
    }
}
#[windows::core::implement(IMMNotificationClient)]
struct EndpointNotifications {
    tx: Sender<crate::engine::Message>,
}
impl IMMNotificationClient_Impl for EndpointNotifications_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> windows::core::Result<()> {
        let _ = self.tx.send(crate::engine::Message::Wake);
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(crate::engine::Message::Wake);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(crate::engine::Message::Wake);
        Ok(())
    }
    fn OnDefaultDeviceChanged(
        &self,
        _: EDataFlow,
        _: ERole,
        _: &PCWSTR,
    ) -> windows::core::Result<()> {
        let _ = self.tx.send(crate::engine::Message::Wake);
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> windows::core::Result<()> {
        let _ = self.tx.send(crate::engine::Message::Wake);
        Ok(())
    }
}
pub struct EventWatcher {
    thread_id: std::sync::Arc<AtomicU32>,
    join: Option<thread::JoinHandle<()>>,
    enumerator: IMMDeviceEnumerator,
    notifications: IMMNotificationClient,
}
impl EventWatcher {
    pub fn start(enumerator: &IMMDeviceEnumerator, tx: Sender<crate::engine::Message>) -> Self {
        let notifications: IMMNotificationClient = EndpointNotifications { tx: tx.clone() }.into();
        if let Err(e) = unsafe { enumerator.RegisterEndpointNotificationCallback(&notifications) } {
            log::warn!("Endpoint notifications unavailable: {e}");
        }
        *WAKE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(tx);
        let thread_id = std::sync::Arc::new(AtomicU32::new(0));
        let id = thread_id.clone();
        let join = thread::spawn(move || unsafe {
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            id.store(GetCurrentThreadId(), Ordering::Release);
            let hooks = [
                (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
                (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND),
                (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
                (EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE),
            ]
            .map(|(min, max)| {
                SetWinEventHook(
                    min,
                    max,
                    None,
                    Some(event),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            });
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            for hook in hooks {
                if !hook.is_invalid() {
                    let _ = UnhookWinEvent(hook);
                }
            }
        });
        Self {
            thread_id,
            join: Some(join),
            enumerator: enumerator.clone(),
            notifications,
        }
    }
}
impl Drop for EventWatcher {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.notifications);
        }
        if let Some(lock) = WAKE.get() {
            if let Ok(mut guard) = lock.lock() {
                *guard = None;
            }
        }
        // Thread creates its message queue before publishing the ID.
        for _ in 0..100 {
            let id = self.thread_id.load(Ordering::Acquire);
            if id != 0 {
                unsafe {
                    let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
                }
                break;
            }
            thread::sleep(std::time::Duration::from_millis(2));
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
