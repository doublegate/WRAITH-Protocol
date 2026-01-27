use crate::utils::windows_definitions::*;

// DJB2 Hash for ASCII strings
pub const fn hash_str(s: &[u8]) -> u32 {
    let mut hash: u32 = 5381;
    let mut i = 0;
    while i < s.len() {
        hash = ((hash << 5).wrapping_add(hash)) + s[i] as u32;
        i += 1;
    }
    hash
}

// Hash for Unicode strings (case insensitive for modules)
pub unsafe fn hash_unicode(s: PWSTR, len: USHORT) -> u32 {
    let mut hash: u32 = 5381;
    let mut i = 0;
    while i < (len / 2) as isize {
        let mut c = *s.offset(i) as u8; // simplified casting
                                        // to_lower
        if c >= b'A' && c <= b'Z' {
            c += 32;
        }
        hash = ((hash << 5).wrapping_add(hash)) + c as u32;
        i += 1;
    }
    hash
}

#[cfg(target_os = "windows")]
pub unsafe fn get_peb() -> *mut PEB {
    let peb: *mut PEB;
    asm!("mov {}, gs:[0x60]", out(reg) peb);
    peb
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn get_peb() -> *mut PEB {
    // Stub for non-windows verification
    core::ptr::null_mut()
}

pub unsafe fn get_module_base(module_hash: u32) -> *const () {
    let peb = get_peb();
    if peb.is_null() {
        return core::ptr::null();
    }

    let ldr = (*peb).Ldr;
    let header_link = &(*ldr).InLoadOrderModuleList as *const LIST_ENTRY;
    let mut current_link = (*header_link).Flink;

    while current_link != header_link as *mut LIST_ENTRY {
        let entry = current_link as *mut LDR_DATA_TABLE_ENTRY;
        let name_str = (*entry).BaseDllName.Buffer;
        let name_len = (*entry).BaseDllName.Length;

        if !name_str.is_null() {
            let h = hash_unicode(name_str, name_len);
            if h == module_hash {
                return (*entry).DllBase as *const ();
            }
        }
        current_link = (*current_link).Flink;
    }
    core::ptr::null()
}

pub unsafe fn resolve_function(module_hash: u32, function_hash: u32) -> *const () {
    let base = get_module_base(module_hash);
    if base.is_null() {
        return core::ptr::null();
    }
    find_export(base as PVOID, function_hash)
}

unsafe fn find_export(base: PVOID, func_hash: u32) -> *const () {
    let dos_header = base as *const IMAGE_DOS_HEADER;
    if (*dos_header).e_magic != 0x5A4D {
        return core::ptr::null();
    }

    let nt_headers = (base as usize + (*dos_header).e_lfanew as usize) as *const IMAGE_NT_HEADERS64;

    // RVA of Export Directory is at offset 0x70 in OptionalHeader (for x64)
    // We access it manually to avoid full struct definition precision issues if any
    // OptionalHeader starts at offset 24 from NT Headers
    // DataDirectory[0] is at offset 112 (0x70) inside OptionalHeader
    // So 24 + 112 = 136 = 0x88

    // Safer: Use the struct if we trust it.
    // Let's assume standard layout.
    // Export Directory RVA is first entry in DataDirectory.
    // We didn't fully define DataDirectory array in struct, so let's do manual pointer arithmetic from OptionalHeader.

    let opt_header = &(*nt_headers).OptionalHeader as *const _ as *const u8;
    let data_dir_offset = 112;
    let export_rva_ptr = opt_header.add(data_dir_offset) as *const ULONG;
    let export_rva = *export_rva_ptr;

    if export_rva == 0 {
        return core::ptr::null();
    }

    let export_dir = (base as usize + export_rva as usize) as *const IMAGE_EXPORT_DIRECTORY;

    let names = (base as usize + (*export_dir).AddressOfNames as usize) as *const ULONG;
    let functions = (base as usize + (*export_dir).AddressOfFunctions as usize) as *const ULONG;
    let ordinals = (base as usize + (*export_dir).AddressOfNameOrdinals as usize) as *const USHORT;

    for i in 0..(*export_dir).NumberOfNames {
        let name_rva = *names.offset(i as isize);
        let name_ptr = (base as usize + name_rva as usize) as *const u8;

        // Hash name
        let s_len = c_strlen(name_ptr);
        let s_slice = core::slice::from_raw_parts(name_ptr, s_len);
        let h = hash_str(s_slice);

        if h == func_hash {
            let ordinal = *ordinals.offset(i as isize);
            let func_rva = *functions.offset(ordinal as isize);
            return (base as usize + func_rva as usize) as *const ();
        }
    }

    core::ptr::null()
}

unsafe fn c_strlen(p: *const u8) -> usize {
    let mut len = 0;
    while *p.add(len) != 0 {
        len += 1;
    }
    len
}
