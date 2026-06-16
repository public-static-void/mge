// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_inventory_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "inventory")]
    unsafe extern "C" {
        fn get_inventory(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_inventory(entity: u32, json_ptr: *const u8, json_len: i32);
        fn add_item_to_inventory(entity: u32, item_json_ptr: *const u8, item_json_len: i32);
        fn remove_item_from_inventory(entity: u32, slot_id: i32);
    }

    unsafe {
        let eid = spawn_entity();

        // Verify no inventory initially (should return -1)
        let mut buf = [0u8; 128];
        let result = get_inventory(eid, buf.as_mut_ptr(), buf.len() as i32);
        if result != -1 {
            return 0;
        }

        // Set inventory with empty slots
        let inv_json = "{\"slots\":[]}";
        set_inventory(eid, inv_json.as_ptr(), inv_json.len() as i32);

        // Get inventory back and verify
        let mut buf2 = [0u8; 128];
        let written = get_inventory(eid, buf2.as_mut_ptr(), buf2.len() as i32);
        if written < 0 {
            return 0;
        }
        let result_str = core::str::from_utf8(&buf2[..written as usize]).unwrap_or("");
        if result_str != "{\"slots\":[]}" {
            return 0;
        }

        // Add an item string
        let item = "\"sword\"";
        add_item_to_inventory(eid, item.as_ptr(), item.len() as i32);

        // Verify item was added
        let mut buf3 = [0u8; 128];
        let written3 = get_inventory(eid, buf3.as_mut_ptr(), buf3.len() as i32);
        if written3 < 0 {
            return 0;
        }
        let result_str3 = core::str::from_utf8(&buf3[..written3 as usize]).unwrap_or("");
        if result_str3 != "{\"slots\":[\"sword\"]}" {
            return 0;
        }

        // Remove the item
        remove_item_from_inventory(eid, 0);

        // Verify removal
        let mut buf4 = [0u8; 128];
        let written4 = get_inventory(eid, buf4.as_mut_ptr(), buf4.len() as i32);
        if written4 < 0 {
            return 0;
        }
        let result_str4 = core::str::from_utf8(&buf4[..written4 as usize]).unwrap_or("");
        if result_str4 != "{\"slots\":[]}" {
            return 0;
        }

        1
    }
}
