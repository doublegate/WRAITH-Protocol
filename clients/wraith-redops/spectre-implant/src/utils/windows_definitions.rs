#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use core::ffi::c_void;

// Basic Types
pub type BOOLEAN = u8;
pub type HANDLE = *mut c_void;
pub type PVOID = *mut c_void;
pub type LPVOID = *mut c_void;
pub type ULONG = u32;
pub type USHORT = u16;
pub type PWSTR = *mut u16;
pub type LPCSTR = *const u8;
pub type FARPROC = *const c_void;

// PEB Related Structures
#[repr(C)]
pub struct UNICODE_STRING {
    pub Length: USHORT,
    pub MaximumLength: USHORT,
    pub Buffer: PWSTR,
}

#[repr(C)]
pub struct PEB {
    pub InheritedAddressSpace: BOOLEAN,
    pub ReadImageFileExecOptions: BOOLEAN,
    pub BeingDebugged: BOOLEAN,
    pub BitField: BOOLEAN,
    pub Mutant: HANDLE,
    pub ImageBaseAddress: PVOID,
    pub Ldr: *mut PEB_LDR_DATA,
    // ... truncated for size, we only need Ldr
}

#[repr(C)]
pub struct PEB_LDR_DATA {
    pub Length: ULONG,
    pub Initialized: BOOLEAN,
    pub SsHandle: HANDLE,
    pub InLoadOrderModuleList: LIST_ENTRY,
    pub InMemoryOrderModuleList: LIST_ENTRY,
    pub InInitializationOrderModuleList: LIST_ENTRY,
}

#[repr(C)]
pub struct LIST_ENTRY {
    pub Flink: *mut LIST_ENTRY,
    pub Blink: *mut LIST_ENTRY,
}

#[repr(C)]
pub struct LDR_DATA_TABLE_ENTRY {
    pub InLoadOrderLinks: LIST_ENTRY,
    pub InMemoryOrderLinks: LIST_ENTRY,
    pub InInitializationOrderLinks: LIST_ENTRY,
    pub DllBase: PVOID,
    pub EntryPoint: PVOID,
    pub SizeOfImage: ULONG,
    pub FullDllName: UNICODE_STRING,
    pub BaseDllName: UNICODE_STRING,
    // ...
}

// PE Header Structures
#[repr(C)]
pub struct IMAGE_DOS_HEADER {
    pub e_magic: USHORT,
    pub e_cblp: USHORT,
    pub e_cp: USHORT,
    pub e_crlc: USHORT,
    pub e_cparhdr: USHORT,
    pub e_minalloc: USHORT,
    pub e_maxalloc: USHORT,
    pub e_ss: USHORT,
    pub e_sp: USHORT,
    pub e_csum: USHORT,
    pub e_ip: USHORT,
    pub e_cs: USHORT,
    pub e_lfarlc: USHORT,
    pub e_ovno: USHORT,
    pub e_res: [USHORT; 4],
    pub e_oemid: USHORT,
    pub e_oeminfo: USHORT,
    pub e_res2: [USHORT; 10],
    pub e_lfanew: i32,
}

#[repr(C)]
pub struct IMAGE_NT_HEADERS64 {
    pub Signature: ULONG,
    pub FileHeader: IMAGE_FILE_HEADER,
    pub OptionalHeader: IMAGE_OPTIONAL_HEADER64,
}

#[repr(C)]
pub struct IMAGE_FILE_HEADER {
    pub Machine: USHORT,
    pub NumberOfSections: USHORT,
    pub TimeDateStamp: ULONG,
    pub PointerToSymbolTable: ULONG,
    pub NumberOfSymbols: ULONG,
    pub SizeOfOptionalHeader: USHORT,
    pub Characteristics: USHORT,
}

#[repr(C)]
pub struct IMAGE_OPTIONAL_HEADER64 {
    pub Magic: USHORT,
    // ... many fields ...
    pub AddressOfEntryPoint: ULONG,
    pub BaseOfCode: ULONG,
    pub ImageBase: u64,
    // ...
    // DataDirectories start at offset 112 usually
    // We skip to DataDirectory
    // But manual offset is risky. Let's define the full struct roughly or use offset logic.
    // Simplifying for clarity: We assume standard offset.
    // Export Directory is index 0.
}

// Minimal needed for finding Export Directory RVA
// Magic (2) + ... + NumberOfRvaAndSizes (4) + DataDirectory (16*8)
// Offset of DataDirectory[0] in OptionalHeader64 is 112 (0x70)

#[repr(C)]
pub struct IMAGE_EXPORT_DIRECTORY {
    pub Characteristics: ULONG,
    pub TimeDateStamp: ULONG,
    pub MajorVersion: USHORT,
    pub MinorVersion: USHORT,
    pub Name: ULONG,
    pub Base: ULONG,
    pub NumberOfFunctions: ULONG,
    pub NumberOfNames: ULONG,
    pub AddressOfFunctions: ULONG,    // RVA from base of image
    pub AddressOfNames: ULONG,        // RVA from base of image
    pub AddressOfNameOrdinals: ULONG, // RVA from base of image
}
