// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_input_api() -> i32 {
    #[link(wasm_import_module = "input")]
    unsafe extern "C" {
        fn get_user_input(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // In test context stdin has no input, so get_user_input returns -1
        let mut buf = [0u8; 64];
        let result = get_user_input(buf.as_mut_ptr(), buf.len() as i32);
        if result != -1 {
            return 0;
        }
        1
    }
}
