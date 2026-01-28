#[cfg(target_os = "windows")]
use crate::utils::api_resolver::{hash_str, resolve_function};
#[cfg(target_os = "windows")]
use crate::utils::windows_definitions::HANDLE;
use alloc::vec::Vec;

#[cfg(target_os = "windows")]
pub struct Screenshot;

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFOHEADER {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: u32,
    biClrImportant: u32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

#[cfg(target_os = "windows")]
impl Screenshot {
    pub fn capture(&self) -> Result<Vec<u8>, ()> {
        // SAFETY: Interacting with Windows GDI APIs.
        unsafe {
            let user32 = hash_str(b"user32.dll");
            let gdi32 = hash_str(b"gdi32.dll");

            // Resolve functions
            let get_desktop_window = resolve_function(user32, hash_str(b"GetDesktopWindow"));
            let get_window_dc = resolve_function(user32, hash_str(b"GetWindowDC"));
            let release_dc = resolve_function(user32, hash_str(b"ReleaseDC"));
            let get_system_metrics = resolve_function(user32, hash_str(b"GetSystemMetrics"));

            let create_compatible_dc = resolve_function(gdi32, hash_str(b"CreateCompatibleDC"));
            let create_compatible_bitmap = resolve_function(gdi32, hash_str(b"CreateCompatibleBitmap"));
            let select_object = resolve_function(gdi32, hash_str(b"SelectObject"));
            let bit_blt = resolve_function(gdi32, hash_str(b"BitBlt"));
            let delete_object = resolve_function(gdi32, hash_str(b"DeleteObject"));
            let delete_dc = resolve_function(gdi32, hash_str(b"DeleteDC"));
            let get_dibits = resolve_function(gdi32, hash_str(b"GetDIBits"));

            if get_desktop_window.is_null() || bit_blt.is_null() { return Err(()); }

            type FnGetDesktopWindow = unsafe extern "system" fn() -> HANDLE;
            type FnGetWindowDC = unsafe extern "system" fn(HANDLE) -> HANDLE;
            type FnReleaseDC = unsafe extern "system" fn(HANDLE, HANDLE) -> i32;
            type FnGetSystemMetrics = unsafe extern "system" fn(i32) -> i32;
            type FnCreateCompatibleDC = unsafe extern "system" fn(HANDLE) -> HANDLE;
            type FnCreateCompatibleBitmap = unsafe extern "system" fn(HANDLE, i32, i32) -> HANDLE;
            type FnSelectObject = unsafe extern "system" fn(HANDLE, HANDLE) -> HANDLE;
            type FnBitBlt = unsafe extern "system" fn(HANDLE, i32, i32, i32, i32, HANDLE, i32, i32, u32) -> i32;
            type FnDeleteObject = unsafe extern "system" fn(HANDLE) -> i32;
            type FnDeleteDC = unsafe extern "system" fn(HANDLE) -> i32;
            type FnGetDIBits = unsafe extern "system" fn(HANDLE, HANDLE, u32, u32, *mut u8, *mut BITMAPINFO, u32) -> i32;

            let get_desktop_window_fn: FnGetDesktopWindow = core::mem::transmute(get_desktop_window);
            let get_window_dc_fn: FnGetWindowDC = core::mem::transmute(get_window_dc);
            let release_dc_fn: FnReleaseDC = core::mem::transmute(release_dc);
            let get_system_metrics_fn: FnGetSystemMetrics = core::mem::transmute(get_system_metrics);
            let create_compatible_dc_fn: FnCreateCompatibleDC = core::mem::transmute(create_compatible_dc);
            let create_compatible_bitmap_fn: FnCreateCompatibleBitmap = core::mem::transmute(create_compatible_bitmap);
            let select_object_fn: FnSelectObject = core::mem::transmute(select_object);
            let bit_blt_fn: FnBitBlt = core::mem::transmute(bit_blt);
            let delete_object_fn: FnDeleteObject = core::mem::transmute(delete_object);
            let delete_dc_fn: FnDeleteDC = core::mem::transmute(delete_dc);
            let get_dibits_fn: FnGetDIBits = core::mem::transmute(get_dibits);

            let h_desktop = get_desktop_window_fn();
            let h_dc = get_window_dc_fn(h_desktop);
            
            let width = get_system_metrics_fn(0); // SM_CXSCREEN
            let height = get_system_metrics_fn(1); // SM_CYSCREEN

            let h_mem_dc = create_compatible_dc_fn(h_dc);
            let h_bitmap = create_compatible_bitmap_fn(h_dc, width, height);
            
            let h_old = select_object_fn(h_mem_dc, h_bitmap);
            
            // SRCCOPY = 0x00CC0020
            bit_blt_fn(h_mem_dc, 0, 0, width, height, h_dc, 0, 0, 0x00CC0020);
            
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // Top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0, // BI_RGB
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0],
            };

            let data_size = (width * height * 4) as usize;
            let mut buffer = alloc::vec![0u8; data_size];
            
            get_dibits_fn(h_mem_dc, h_bitmap, 0, height as u32, buffer.as_mut_ptr(), &mut bmi, 0); // DIB_RGB_COLORS = 0

            // Cleanup
            select_object_fn(h_mem_dc, h_old);
            delete_object_fn(h_bitmap);
            delete_dc_fn(h_mem_dc);
            release_dc_fn(h_desktop, h_dc);
            
            Ok(buffer)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub struct Screenshot;

#[cfg(not(target_os = "windows"))]
impl Screenshot {
    pub fn capture(&self) -> Result<Vec<u8>, ()> {
        Ok(alloc::vec::Vec::from(b"Screenshot not supported on Linux yet" as &[u8]))
    }
}
