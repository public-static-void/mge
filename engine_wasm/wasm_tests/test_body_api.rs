// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_body_api() -> i32 {
    #[link(wasm_import_module = "entity")]
    unsafe extern "C" {
        fn spawn_entity() -> u32;
    }

    #[link(wasm_import_module = "component")]
    unsafe extern "C" {
        fn set_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            json_ptr: *const u8,
            json_len: i32,
        );
        fn get_component(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    #[link(wasm_import_module = "body")]
    unsafe extern "C" {
        fn get_body(entity: u32, out_ptr: *mut u8, out_len: i32) -> i32;
        fn set_body(entity: u32, json_ptr: *const u8, json_len: i32);
        fn add_body_part(entity: u32, part_ptr: *const u8, part_len: i32);
        fn get_body_part(
            entity: u32,
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        let eid = spawn_entity();

        // Set body
        let body_json = "{\"parts\":[{\"name\":\"torso\",\"children\":[]}]}";
        set_body(eid, body_json.as_ptr(), body_json.len() as i32);

        // Get body back
        let mut buf = [0u8; 4096];
        let written = get_body(eid, buf.as_mut_ptr(), buf.len() as i32);
        if written < 0 { return 0; }

        // Add body part
        let part_json = "{\"name\":\"head\",\"children\":[]}";
        add_body_part(eid, part_json.as_ptr(), part_json.len() as i32);

        // Get body part by name
        let part_name = "head";
        let mut buf2 = [0u8; 4096];
        let written2 = get_body_part(eid, part_name.as_ptr(), part_name.len() as i32, buf2.as_mut_ptr(), buf2.len() as i32);
        if written2 < 0 { return 0; }

        1
    }
}
