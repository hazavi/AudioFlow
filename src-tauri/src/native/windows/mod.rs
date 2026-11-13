use crate::{
    models::*,
    native::{AudioRoutingProvider, Platform, Routes},
};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::Audio::*,
        System::{Com::*, Threading::*},
        UI::WindowsAndMessaging::*,
    },
};
mod policy;
use windows::core::BOOL;
#[cfg(test)]
mod hardware;
mod icon;
mod tone;
mod tracking;
pub use tracking::EventWatcher;

pub struct WindowsPlatform {
    enumerator: IMMDeviceEnumerator,
    policy: Result<policy::Policy, String>,
}
pub struct ComApartment;
impl ComApartment {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.map_err(|e| e.to_string())?;
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}
fn err(e: windows::core::Error) -> String {
    e.to_string()
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn from_wide(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())])
}
unsafe fn device_id(device: &IMMDevice) -> Result<String, String> {
    let raw = unsafe { device.GetId() }.map_err(err)?;
    let result = unsafe { raw.to_string() }.map_err(|e| e.to_string());
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    result
}
impl WindowsPlatform {
    pub fn new() -> Result<Self, String> {
        let enumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(err)?;
        Ok(Self {
            enumerator,
            policy: policy::Policy::new(),
        })
    }
    pub fn enumerator(&self) -> &IMMDeviceEnumerator {
        &self.enumerator
    }
}
impl AudioRoutingProvider for WindowsPlatform {
    fn list_output_devices(&self) -> Result<Vec<Device>, String> {
        unsafe {
            use windows::Win32::{
                Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
                System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc},
            };
            let default = self
                .enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .ok()
                .and_then(|d| device_id(&d).ok());
            let collection = self
                .enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(err)?;
            let mut result = vec![];
            for i in 0..collection.GetCount().map_err(err)? {
                let device = collection.Item(i).map_err(err)?;
                let id = device_id(&device)?;
                let store = device.OpenPropertyStore(STGM_READ).map_err(err)?;
                let mut value = store.GetValue(&PKEY_Device_FriendlyName).map_err(err)?;
                let name = match PropVariantToStringAlloc(&value) {
                    Ok(raw) => {
                        let name = raw.to_string().unwrap_or_else(|_| id.clone());
                        CoTaskMemFree(Some(raw.0.cast()));
                        name
                    }
                    Err(_) => id.clone(),
                };
                let _ = PropVariantClear(&mut value);
                result.push(Device {
                    is_default: default.as_ref() == Some(&id),
                    id,
                    name,
                });
            }
            result.sort_by_key(|d| (!d.is_default, d.name.clone()));
            Ok(result)
        }
    }
    fn list_audio_sessions(&self, devices: &[Device]) -> Result<Vec<Session>, String> {
        unsafe {
            use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
            let mut result = vec![];
            for endpoint in devices {
                let id = wide(&endpoint.id);
                let device = self
                    .enumerator
                    .GetDevice(PCWSTR(id.as_ptr()))
                    .map_err(err)?;
                let manager: IAudioSessionManager2 =
                    device.Activate(CLSCTX_ALL, None).map_err(err)?;
                let sessions = manager.GetSessionEnumerator().map_err(err)?;
                for i in 0..sessions.GetCount().map_err(err)? {
                    let Ok(session) = sessions.GetSession(i) else {
                        continue;
                    };
                    let Ok(control) = session.cast::<IAudioSessionControl2>() else {
                        continue;
                    };
                    let Ok(pid) = control.GetProcessId() else {
                        continue;
                    };
                    if pid == 0 || pid == std::process::id() {
                        continue;
                    }
                    let state = session.GetState().map_err(err)?;
                    if state == AudioSessionStateExpired {
                        continue;
                    }
                    let peak = session
                        .cast::<IAudioMeterInformation>()
                        .ok()
                        .and_then(|m| m.GetPeakValue().ok());
                    result.push(Session {
                        pid,
                        endpoint_id: endpoint.id.clone(),
                        active: state == AudioSessionStateActive,
                        peak,
                    });
                }
            }
            Ok(result)
        }
    }
    fn get_current_route(&self, pid: u32) -> Result<Routes, String> {
        self.policy.as_ref().map_err(Clone::clone)?.get(pid)
    }
    fn set_process_output(&self, pid: u32, endpoint: Option<&str>) -> Result<(), String> {
        self.policy
            .as_ref()
            .map_err(Clone::clone)?
            .set(pid, endpoint)
    }
    fn restore_routes(&self, pid: u32, routes: &Routes) -> Result<(), String> {
        self.policy
            .as_ref()
            .map_err(Clone::clone)?
            .restore(pid, routes)
    }
    fn supports_process_routing(&self) -> bool {
        self.policy.is_ok()
    }
    fn get_last_error(&self) -> Option<String> {
        self.policy.as_ref().err().cloned()
    }
}
impl Platform for WindowsPlatform {
    fn icon(&self, executable: &str) -> Option<String> {
        icon::extract(executable)
    }
    fn monitors(&self) -> Result<Vec<Monitor>, String> {
        unsafe {
            let mut result = vec![];
            let mut index = 0;
            loop {
                let mut adapter = DISPLAY_DEVICEW {
                    cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                    ..Default::default()
                };
                if !EnumDisplayDevicesW(None, index, &mut adapter, 0).as_bool() {
                    break;
                }
                index += 1;
                if !adapter
                    .StateFlags
                    .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
                {
                    continue;
                }
                let mut mode = DEVMODEW {
                    dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                    ..Default::default()
                };
                if !EnumDisplaySettingsW(
                    PCWSTR(adapter.DeviceName.as_ptr()),
                    ENUM_CURRENT_SETTINGS,
                    &mut mode,
                )
                .as_bool()
                {
                    continue;
                }
                let mut display = DISPLAY_DEVICEW {
                    cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                    ..Default::default()
                };
                let found = EnumDisplayDevicesW(
                    PCWSTR(adapter.DeviceName.as_ptr()),
                    0,
                    &mut display,
                    EDD_GET_DEVICE_INTERFACE_NAME,
                )
                .as_bool();
                let device_name = from_wide(&adapter.DeviceName);
                let id = if found {
                    from_wide(&display.DeviceID)
                } else {
                    device_name.clone()
                };
                let name = if found {
                    from_wide(&display.DeviceString)
                } else {
                    from_wide(&adapter.DeviceString)
                };
                let position = mode.Anonymous1.Anonymous2.dmPosition;
                result.push(Monitor {
                    id,
                    name,
                    device_name,
                    bounds: Bounds {
                        x: position.x,
                        y: position.y,
                        width: mode.dmPelsWidth as i32,
                        height: mode.dmPelsHeight as i32,
                    },
                    primary: adapter.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE),
                    refresh_rate: Some(mode.dmDisplayFrequency),
                    scale: None,
                });
            }
            // QueryDisplayConfig exposes EDID-friendly names even when GDI says Generic PnP Monitor.
            use windows::Win32::Devices::Display::*;
            for _ in 0..3 {
                let (mut path_count, mut mode_count) = (0, 0);
                if GetDisplayConfigBufferSizes(
                    QDC_ONLY_ACTIVE_PATHS,
                    &mut path_count,
                    &mut mode_count,
                )
                .is_err()
                {
                    break;
                }
                let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
                let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
                let status = QueryDisplayConfig(
                    QDC_ONLY_ACTIVE_PATHS,
                    &mut path_count,
                    paths.as_mut_ptr(),
                    &mut mode_count,
                    modes.as_mut_ptr(),
                    None,
                );
                if status == ERROR_INSUFFICIENT_BUFFER {
                    continue;
                }
                if status.is_err() {
                    break;
                }
                for path in paths.iter().take(path_count as usize) {
                    let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                    source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                        size: std::mem::size_of_val(&source) as u32,
                        adapterId: path.sourceInfo.adapterId,
                        id: path.sourceInfo.id,
                    };
                    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                    target.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                        size: std::mem::size_of_val(&target) as u32,
                        adapterId: path.targetInfo.adapterId,
                        id: path.targetInfo.id,
                    };
                    if DisplayConfigGetDeviceInfo(&mut source.header) == 0
                        && DisplayConfigGetDeviceInfo(&mut target.header) == 0
                    {
                        if let Some(monitor) = result.iter_mut().find(|m| {
                            m.device_name
                                .eq_ignore_ascii_case(&from_wide(&source.viewGdiDeviceName))
                        }) {
                            let friendly = from_wide(&target.monitorFriendlyDeviceName);
                            if !friendly.is_empty() {
                                monitor.name = friendly;
                            }
                        }
                    }
                }
                break;
            }
            for monitor in &mut result {
                let point = POINT {
                    x: monitor.bounds.x + monitor.bounds.width / 2,
                    y: monitor.bounds.y + monitor.bounds.height / 2,
                };
                let handle = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
                let (mut x, mut y) = (0, 0);
                if windows::Win32::UI::HiDpi::GetDpiForMonitor(
                    handle,
                    windows::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
                    &mut x,
                    &mut y,
                )
                .is_ok()
                {
                    monitor.scale = Some(x * 100 / 96);
                }
            }
            result.sort_by_key(|m| (!m.primary, m.bounds.x, m.bounds.y));
            Ok(result)
        }
    }
    fn windows(&self) -> Result<Vec<AppWindow>, String> {
        unsafe extern "system" fn callback(hwnd: HWND, param: LPARAM) -> BOOL {
            unsafe {
                if !IsWindowVisible(hwnd).as_bool()
                    || IsIconic(hwnd).as_bool()
                    || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0
                    || GetWindow(hwnd, GW_OWNER).is_ok_and(|w| !w.is_invalid())
                {
                    return BOOL(1);
                }
                let mut cloaked = 0u32;
                #[link(name = "dwmapi")]
                unsafe extern "system" {
                    fn DwmGetWindowAttribute(
                        hwnd: HWND,
                        attribute: u32,
                        value: *mut core::ffi::c_void,
                        size: u32,
                    ) -> windows::core::HRESULT;
                }
                let _ = DwmGetWindowAttribute(hwnd, 14, (&mut cloaked as *mut u32).cast(), 4);
                if cloaked != 0 {
                    return BOOL(1);
                }
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid == 0 || pid == std::process::id() {
                    return BOOL(1);
                }
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_err() {
                    return BOOL(1);
                }
                if rect.right <= rect.left || rect.bottom <= rect.top {
                    return BOOL(1);
                }
                let list = &mut *(param.0 as *mut Vec<AppWindow>);
                list.push(AppWindow {
                    handle: hwnd.0 as isize,
                    pid,
                    bounds: Bounds {
                        x: rect.left,
                        y: rect.top,
                        width: rect.right - rect.left,
                        height: rect.bottom - rect.top,
                    },
                    focused: GetForegroundWindow() == hwnd,
                });
                BOOL(1)
            }
        }
        let mut result: Vec<AppWindow> = vec![];
        unsafe {
            EnumWindows(
                Some(callback),
                LPARAM((&mut result as *mut Vec<AppWindow>) as isize),
            )
        }
        .map_err(err)?;
        Ok(result)
    }
    fn process(&self, pid: u32) -> Result<(String, u64), String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).map_err(err)?;
            let result = (|| {
                let mut path = vec![0u16; 32768];
                let mut len = path.len() as u32;
                QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    windows::core::PWSTR(path.as_mut_ptr()),
                    &mut len,
                )
                .map_err(err)?;
                let mut created = FILETIME::default();
                let mut exit = FILETIME::default();
                let mut kernel = FILETIME::default();
                let mut user = FILETIME::default();
                GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user)
                    .map_err(err)?;
                Ok((
                    String::from_utf16_lossy(&path[..len as usize]),
                    (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime),
                ))
            })();
            let _ = CloseHandle(handle);
            result
        }
    }
    fn parents(&self) -> std::collections::HashMap<u32, u32> {
        use windows::Win32::System::Diagnostics::ToolHelp::*;
        let mut result = std::collections::HashMap::new();
        unsafe {
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return result;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    result.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
        }
        result
    }
    fn test_speaker(&self, endpoint: &str) -> Result<(), String> {
        tone::play(&self.enumerator, endpoint)
    }
}
