// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_turn_api() -> i32 {
    #[link(wasm_import_module = "turn")]
    unsafe extern "C" {
        fn tick();
        fn get_turn() -> i32;
    }

    unsafe {
        let initial = get_turn();
        if initial != 0 {
            return 0;
        }

        tick();
        tick();
        tick();

        let after = get_turn();
        if after != 3 {
            return 0;
        }

        1
    }
}
