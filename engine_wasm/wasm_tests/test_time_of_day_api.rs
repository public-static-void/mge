// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_time_of_day_api() -> i32 {
    #[link(wasm_import_module = "time_of_day")]
    unsafe extern "C" {
        fn get_time_of_day(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // Call get_time_of_day and verify it returns non-empty data
        let mut out_buf = [0u8; 64];
        let written = get_time_of_day(out_buf.as_mut_ptr(), out_buf.len() as i32);
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if result.is_empty() {
            return 0;
        }
        // Verify it contains hour and minute fields
        if !result.contains("hour") || !result.contains("minute") {
            return 0;
        }

        1
    }
}
