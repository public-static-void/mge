// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_mode_api() -> i32 {
    #[link(wasm_import_module = "mode")]
    unsafe extern "C" {
        fn set_mode(ptr: *const u8, len: i32);
        fn get_mode(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_available_modes(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // Set mode to "exploration"
        let mode = "exploration";
        set_mode(mode.as_ptr(), mode.len() as i32);

        // Get mode back and verify
        let mut out_buf = [0u8; 64];
        let written = get_mode(out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if result != "exploration" {
            return 0;
        }

        // Verify get_available_modes returns a non-empty JSON array
        let mut out_buf2 = [0u8; 256];
        let written2 = get_available_modes(out_buf2.as_mut_ptr(), out_buf2.len() as i32);
        if written2 <= 0 {
            return 0;
        }

        1
    }
}
