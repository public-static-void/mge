// This file is compiled to WASM and loaded by the Rust host test harness.
// It exports mge_worldgen_generate so the host can discover it.

#![no_std]
#![no_main]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn mge_worldgen_generate(_params_ptr: i32, _params_len: i32) -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn test_export_discovery() -> i32 {
    1
}
