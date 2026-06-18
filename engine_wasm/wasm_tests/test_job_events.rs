// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_job_events() -> i32 {
    #[link(wasm_import_module = "job_events")]
    unsafe extern "C" {
        fn get_log(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_by_type(
            type_ptr: *const u8,
            type_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn get_since(tick: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn poll_bus(
            entity_id: i32,
            type_ptr: *const u8,
            type_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
        fn clear() -> i32;
    }

    unsafe {
        // Clear should return 0
        let clear_result = clear();
        if clear_result != 0 {
            return 0;
        }

        // Get log — should be "[]" (empty array) after clear
        let mut log_buf = [0u8; 512];
        let log_written = get_log(log_buf.as_mut_ptr(), log_buf.len() as i32);
        if log_written < 0 {
            return 0;
        }
        let log_str = core::str::from_utf8(&log_buf[..log_written as usize]).unwrap_or("");
        if log_str != "[]" {
            return 0;
        }

        // Get by type — should return "[]" for unknown type
        let event_type = "test";
        let mut type_buf = [0u8; 512];
        let type_written = get_by_type(
            event_type.as_ptr(),
            event_type.len() as i32,
            type_buf.as_mut_ptr(),
            type_buf.len() as i32,
        );
        if type_written < 0 {
            return 0;
        }
        let type_str = core::str::from_utf8(&type_buf[..type_written as usize]).unwrap_or("");
        if type_str != "[]" {
            return 0;
        }

        // Get since tick 0 — should return "[]" when no events exist
        let mut since_buf = [0u8; 512];
        let since_written = get_since(0, since_buf.as_mut_ptr(), since_buf.len() as i32);
        if since_written < 0 {
            return 0;
        }
        let since_str = core::str::from_utf8(&since_buf[..since_written as usize]).unwrap_or("");
        if since_str != "[]" {
            return 0;
        }

        // Poll bus for an entity — should return JSON array (not -1) even when empty
        let bus_type = "job_completed";
        let mut bus_buf = [0u8; 512];
        let bus_written = poll_bus(
            0,
            bus_type.as_ptr(),
            bus_type.len() as i32,
            bus_buf.as_mut_ptr(),
            bus_buf.len() as i32,
        );
        if bus_written < 0 {
            return 0;
        }
        let bus_str = core::str::from_utf8(&bus_buf[..bus_written as usize]).unwrap_or("");
        if !bus_str.starts_with('[') {
            return 0;
        }

        // Clear again after operations — should still return 0
        let clear2 = clear();
        if clear2 != 0 {
            return 0;
        }

        1
    }
}
