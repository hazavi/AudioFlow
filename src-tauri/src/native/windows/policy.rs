//! Internal Windows AudioPolicyConfig ABI. Kept here so a Windows update can disable
//! routing without affecting discovery. No global-default setter or clear-all API.
use windows::core::{IInspectable, IInspectable_Vtbl, Interface, GUID, HRESULT, HSTRING};

const PREFIX: &str = "\\\\?\\SWD#MMDEVAPI#";
const SUFFIX: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";

#[repr(C)]
struct PolicyVtbl {
    base: IInspectable_Vtbl,
    reserved: [usize; 19],
    set: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u32,
        i32,
        i32,
        *mut core::ffi::c_void,
    ) -> HRESULT,
    get: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u32,
        i32,
        i32,
        *mut *mut core::ffi::c_void,
    ) -> HRESULT,
}
pub struct Policy {
    object: IInspectable,
}
impl Policy {
    pub fn new() -> Result<Self, String> {
        // Query the exact IID; never invoke a guessed vtable from a different interface.
        for guid in [
            GUID::from_u128(0xab3d4648_e242_459f_b02f_541c70306324),
            GUID::from_u128(0x2a59116d_6c4f_45e0_a74f_707e3fef9258),
        ] {
            let mut raw = std::ptr::null_mut();
            #[link(name = "runtimeobject")]
            unsafe extern "system" {
                #[link_name = "RoGetActivationFactory"]
                fn activation(
                    class: *mut core::ffi::c_void,
                    iid: *const GUID,
                    result: *mut *mut core::ffi::c_void,
                ) -> HRESULT;
            }
            let class = HSTRING::from("Windows.Media.Internal.AudioPolicyConfig");
            if unsafe { activation(std::mem::transmute_copy(&class), &guid, &mut raw) }.is_ok()
                && !raw.is_null()
            {
                return Ok(Self {
                    object: unsafe { IInspectable::from_raw(raw) },
                });
            }
        }
        Err("This Windows build does not expose a compatible per-application audio policy interface. Discovery still works.".into())
    }
    fn vtable(&self) -> &PolicyVtbl {
        unsafe { &**(self.object.as_raw() as *const *const PolicyVtbl) }
    }
    pub fn get(&self, pid: u32) -> Result<crate::native::Routes, String> {
        let mut result = [None, None, None];
        for role in 0..3 {
            let mut raw = std::ptr::null_mut();
            unsafe { (self.vtable().get)(self.object.as_raw(), pid, 0, role, &mut raw).ok() }
                .map_err(|e| format!("Read app route (PID {pid}, role {role}): {e}"))?;
            if !raw.is_null() {
                let value: HSTRING = unsafe { std::mem::transmute(raw) };
                let value = value.to_string_lossy();
                if !value.is_empty() {
                    result[role as usize] = Some(
                        value
                            .strip_prefix(PREFIX)
                            .unwrap_or(&value)
                            .strip_suffix(SUFFIX)
                            .unwrap_or(value.strip_prefix(PREFIX).unwrap_or(&value))
                            .into(),
                    );
                }
            }
        }
        Ok(result)
    }
    pub fn set_role(&self, pid: u32, role: i32, endpoint: Option<&str>) -> Result<(), String> {
        let value = endpoint.map(|id| HSTRING::from(format!("{PREFIX}{id}{SUFFIX}")));
        let raw = value
            .as_ref()
            .map(|v| unsafe { std::mem::transmute_copy(v) })
            .unwrap_or(std::ptr::null_mut());
        unsafe { (self.vtable().set)(self.object.as_raw(), pid, 0, role, raw).ok() }
            .map_err(|e| format!("Set app route (PID {pid}, role {role}): {e}"))
    }
    pub fn restore(&self, pid: u32, routes: &crate::native::Routes) -> Result<(), String> {
        for (role, endpoint) in routes.iter().enumerate() {
            self.set_role(pid, role as i32, endpoint.as_deref())?;
        }
        Ok(())
    }
    pub fn set(&self, pid: u32, endpoint: Option<&str>) -> Result<(), String> {
        let original = self.get(pid)?;
        for role in 0..3 {
            if let Err(e) = self.set_role(pid, role, endpoint) {
                let _ = self.restore(pid, &original);
                return Err(e);
            }
        }
        let expected = [
            endpoint.map(str::to_owned),
            endpoint.map(str::to_owned),
            endpoint.map(str::to_owned),
        ];
        if self.get(pid)? != expected {
            let _ = self.restore(pid, &original);
            return Err("Windows did not retain the requested app output preference.".into());
        }
        Ok(())
    }
}
