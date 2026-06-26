// This file is compiled to WASM and loaded by the Rust host test harness.
// Tests the loot table API (define_table, roll, has_table, table_names, remove_table).

#[unsafe(no_mangle)]
pub extern "C" fn test_loot_api() -> i32 {
    #[link(wasm_import_module = "loot")]
    unsafe extern "C" {
        fn define_table(name_ptr: *const u8, name_len: i32, entries_ptr: *const u8, entries_len: i32);
        fn roll(name_ptr: *const u8, name_len: i32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn has_table(name_ptr: *const u8, name_len: i32) -> i32;
        fn table_names(out_ptr: *mut u8, out_len: i32) -> i32;
        fn remove_table(name_ptr: *const u8, name_len: i32);
    }

    unsafe {
        // Step 1: Verify has_table returns 0 for undefined table
        let name1 = "monster_drops";
        let result = has_table(name1.as_ptr(), name1.len() as i32);
        if result != 0 {
            return 0;
        }

        // Step 2: Define a loot table
        let name2 = "monster_drops";
        let entries = "[{\"item_id\":\"gold\",\"weight\":100,\"min_count\":1,\"max_count\":3},{\"item_id\":\"silver\",\"weight\":50,\"min_count\":1,\"max_count\":2}]";
        define_table(
            name2.as_ptr(),
            name2.len() as i32,
            entries.as_ptr(),
            entries.len() as i32,
        );

        // Step 3: Verify has_table returns 1
        let result = has_table(name2.as_ptr(), name2.len() as i32);
        if result != 1 {
            return 0;
        }

        // Step 4: Roll on the table
        let mut out_buf = [0u8; 512];
        let written = roll(
            name2.as_ptr(),
            name2.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written < 0 {
            return 0;
        }
        let result_str = core::str::from_utf8(&out_buf[..written as usize]).unwrap_or("");
        if result_str.is_empty() || result_str == "[]" || !result_str.starts_with('[') {
            return 0;
        }

        // Step 5: Get table names
        let mut names_buf = [0u8; 256];
        let names_written = table_names(names_buf.as_mut_ptr(), names_buf.len() as i32);
        if names_written <= 0 {
            return 0;
        }
        let names_str =
            core::str::from_utf8(&names_buf[..names_written as usize]).unwrap_or("");
        if !names_str.contains("monster_drops") {
            return 0;
        }

        // Step 6: Remove the table
        remove_table(name2.as_ptr(), name2.len() as i32);
        let result = has_table(name2.as_ptr(), name2.len() as i32);
        if result != 0 {
            return 0;
        }

        // Step 7: Roll on undefined table should return -1
        let mut out_buf2 = [0u8; 64];
        let written2 = roll(
            name2.as_ptr(),
            name2.len() as i32,
            out_buf2.as_mut_ptr(),
            out_buf2.len() as i32,
        );
        if written2 != -1 {
            return 0;
        }

        1
    }
}
