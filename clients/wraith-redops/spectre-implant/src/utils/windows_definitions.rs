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

#[repr(C)]
#[allow(non_snake_case)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

impl GUID {
    pub const fn new(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Self {
        Self { Data1: d1, Data2: d2, Data3: d3, Data4: d4 }
    }
}

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

#[repr(C)]
pub struct THREADENTRY32 {
    pub dwSize: ULONG,
    pub cntUsage: ULONG,
    pub th32ThreadID: ULONG,
    pub th32OwnerProcessID: ULONG,
    pub tpBasePri: i32,
    pub tpDeltaPri: i32,
    pub dwFlags: ULONG,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct PROCESSENTRY32 {
    pub dwSize: ULONG,
    pub cntUsage: ULONG,
    pub th32ProcessID: ULONG,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: ULONG,
    pub cntThreads: ULONG,
    pub th32ParentProcessID: ULONG,
    pub pcPriClassBase: i32,
    pub dwFlags: ULONG,
    pub szExeFile: [u8; 260],
}

#[repr(C, align(16))]
pub struct CONTEXT {
    pub P1Home: u64,
    pub P2Home: u64,
    pub P3Home: u64,
    pub P4Home: u64,
    pub P5Home: u64,
    pub P6Home: u64,
    pub ContextFlags: u32,
    pub MxCsr: u32,
    pub SegCs: u16,
    pub SegDs: u16,
    pub SegEs: u16,
    pub SegFs: u16,
    pub SegGs: u16,
    pub SegSs: u16,
    pub EFlags: u32,
    pub Dr0: u64,
    pub Dr1: u64,
    pub Dr2: u64,
    pub Dr3: u64,
    pub Dr6: u64,
    pub Dr7: u64,
    pub Rax: u64,
    pub Rcx: u64,
    pub Rdx: u64,
    pub Rbx: u64,
    pub Rsp: u64,
    pub Rbp: u64,
    pub Rsi: u64,
    pub Rdi: u64,
    pub R8:  u64,
    pub R9:  u64,
    pub R10: u64,
    pub R11: u64,
    pub R12: u64,
    pub R13: u64,
    pub R14: u64,
    pub R15: u64,
    pub Rip: u64,
    pub Header: [u128; 2],
    pub Legacy: [u128; 8],
    pub Xmm0: u128,
    pub Xmm1: u128,
    pub Xmm2: u128,
    pub Xmm3: u128,
    pub Xmm4: u128,
    pub Xmm5: u128,
    pub Xmm6: u128,
    pub Xmm7: u128,
    pub Xmm8: u128,
    pub Xmm9: u128,
    pub Xmm10: u128,
    pub Xmm11: u128,
    pub Xmm12: u128,
    pub Xmm13: u128,
    pub Xmm14: u128,
    pub Xmm15: u128,
    pub VectorRegister: [u128; 26],
    pub VectorControl: u64,
    pub DebugControl: u64,
    pub LastBranchToRip: u64,
    pub LastExceptionToRip: u64,
    pub LastExceptionFromRip: u64,
}

#[repr(C)]
pub struct PROCESS_HEAP_ENTRY {
    pub lpData: PVOID,
    pub cbData: u32,
    pub cbOverhead: u8,
    pub iRegionIndex: u8,
    pub wFlags: u16,
    pub u: [u8; 16],
}

#[repr(C)]
pub struct MEMORY_BASIC_INFORMATION {
    pub BaseAddress: PVOID,
    pub AllocationBase: PVOID,
    pub AllocationProtect: ULONG,
    pub RegionSize: usize,
    pub State: ULONG,
    pub Protect: ULONG,
    pub Type: ULONG,
}

#[repr(C)]
pub struct PROCESS_BASIC_INFORMATION {
    pub ExitStatus: i32,
    pub PebBaseAddress: PVOID,
    pub AffinityMask: usize,
    pub BasePriority: i32,
    pub UniqueProcessId: usize,
    pub InheritedFromUniqueProcessId: usize,
}

#[repr(C)]
pub struct ITaskServiceVtbl {
    pub QueryInterface: PVOID,
    pub AddRef: PVOID,
    pub Release: PVOID,
    pub GetTargetServer: PVOID,
    pub GetConnected: PVOID,
    pub GetConnectedDomain: PVOID,
    pub GetConnectedUser: PVOID,
    pub GetHighestVersion: PVOID,
    pub Connect: unsafe extern "system" fn(*mut ITaskService, *mut u16, *mut u16, *mut u16, *mut u16) -> i32,
    pub GetFolder: unsafe extern "system" fn(*mut ITaskService, *const u16, *mut *mut c_void) -> i32,
    pub NewTask: unsafe extern "system" fn(*mut ITaskService, u32, *mut *mut ITaskDefinition) -> i32,
}

#[repr(C)]
pub struct ITaskService {
    pub vtbl: *const ITaskServiceVtbl,
}

#[repr(C)]
pub struct ITaskFolderVtbl {
    pub QueryInterface: PVOID,
    pub AddRef: PVOID,
    pub Release: PVOID,
    pub GetName: PVOID,
    pub GetPath: PVOID,
    pub GetFolder: PVOID,
    pub GetFolders: PVOID,
    pub CreateFolder: PVOID,
    pub DeleteFolder: PVOID,
    pub GetTask: PVOID,
    pub GetTasks: PVOID,
    pub DeleteTask: PVOID,
    pub RegisterTask: PVOID,
    pub RegisterTaskDefinition: unsafe extern "system" fn(*mut ITaskFolder, *const u16, *mut ITaskDefinition, i32, *mut u16, *mut u16, i32, *mut u16, *mut *mut c_void) -> i32,
}

#[repr(C)]
pub struct ITaskFolder {
    pub vtbl: *const ITaskFolderVtbl,
}

#[repr(C)]
pub struct ITaskDefinitionVtbl {
    pub QueryInterface: PVOID,
    pub AddRef: PVOID,
    pub Release: PVOID,
    pub get_RegistrationInfo: PVOID,
    pub put_RegistrationInfo: PVOID,
    pub get_Triggers: PVOID,
    pub put_Triggers: PVOID,
    pub get_Settings: PVOID,
    pub put_Settings: PVOID,
    pub get_Data: PVOID,
    pub put_Data: PVOID,
    pub get_Principal: PVOID,
    pub put_Principal: PVOID,
    pub get_Actions: unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut IActionCollection) -> i32,
}

#[repr(C)]
pub struct ITaskDefinition {
    pub vtbl: *const ITaskDefinitionVtbl,
}

#[repr(C)]
pub struct IActionCollectionVtbl {
    pub QueryInterface: PVOID,
    pub AddRef: PVOID,
    pub Release: PVOID,
    pub get_Count: PVOID,
    pub get_Item: PVOID,
    pub get__NewEnum: PVOID,
    pub Create: unsafe extern "system" fn(*mut IActionCollection, i32, *mut *mut IExecAction) -> i32,
}

#[repr(C)]
pub struct IActionCollection {
    pub vtbl: *const IActionCollectionVtbl,
}

#[repr(C)]
pub struct IExecActionVtbl {
    pub QueryInterface: unsafe extern "system" fn(*mut IExecAction, *const GUID, *mut *mut c_void) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut IExecAction) -> u32,
    pub Release: unsafe extern "system" fn(*mut IExecAction) -> u32,
    pub get_Id: PVOID,
    pub put_Id: PVOID,
    pub get_Path: PVOID,
    pub put_Path: unsafe extern "system" fn(*mut IExecAction, *const u16) -> i32,
    pub get_Arguments: PVOID,
    pub put_Arguments: unsafe extern "system" fn(*mut IExecAction, *const u16) -> i32,
}

#[repr(C)]
pub struct IExecAction {
    pub vtbl: *const IExecActionVtbl,
}

#[repr(C)]
pub struct MINIDUMP_CALLBACK_INFORMATION {
    pub CallbackRoutine: PVOID,
    pub CallbackParam: PVOID,
}

#[repr(C)]
pub struct MINIDUMP_CALLBACK_INPUT {
    pub ProcessId: ULONG,
    pub ProcessHandle: HANDLE,
    pub CallbackType: u32,
    pub Io: MINIDUMP_IO_CALLBACK,
}

#[repr(C)]
pub struct MINIDUMP_IO_CALLBACK {
    pub Handle: HANDLE,
    pub Offset: u64,
    pub Buffer: PVOID,
    pub BufferBytes: u32,
}

#[repr(C)]
pub struct MINIDUMP_CALLBACK_OUTPUT {
    pub Status: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_context_size() {
        // x64 CONTEXT should be 1232 bytes (0x4D0)
        // This confirms the fields are correctly packed and aligned
        assert_eq!(size_of::<CONTEXT>(), 1232);
    }
}
