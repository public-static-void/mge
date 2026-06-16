// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_save_load_api() -> i32 {
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

    #[link(wasm_import_module = "save_load")]
    unsafe extern "C" {
        fn save_to_file(path_ptr: *const u8, path_len: i32);
        fn load_from_file(path_ptr: *const u8, path_len: i32);
    }

    unsafe {
        let eid = spawn_entity();

        // Set a component on the entity
        let comp_name = "Health";
        let comp_json = "{\"hp\": 100}";
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            comp_json.as_ptr(),
            comp_json.len() as i32,
        );

        // Verify component was set
        let mut buf = [0u8; 64];
        let get_name = "Health";
        let written = get_component(
            eid,
            get_name.as_ptr(),
            get_name.len() as i32,
            buf.as_mut_ptr(),
            buf.len() as i32,
        );
        if written <= 0 {
            return 0;
        }
        let result = core::str::from_utf8(&buf[..written as usize]).unwrap_or("");
        if result != "{\"hp\":100}" {
            return 0;
        }

        // Save to a temp path (host will resolve)
        let save_path = "/tmp/wasm_save_test.json";
        save_to_file(save_path.as_ptr(), save_path.len() as i32);

        // Modify world state: spawn another entity
        let _eid2 = spawn_entity();

        // Load back the saved state
        load_from_file(save_path.as_ptr(), save_path.len() as i32);

        // After reload, there should be only 1 entity again
        // We check by verifying the Health component is present on eid
        let mut buf2 = [0u8; 64];
        let written2 = get_component(
            eid,
            get_name.as_ptr(),
            get_name.len() as i32,
            buf2.as_mut_ptr(),
            buf2.len() as i32,
        );
        if written2 <= 0 {
            return 0;
        }
        let result2 = core::str::from_utf8(&buf2[..written2 as usize]).unwrap_or("");
        if result2 != "{\"hp\":100}" {
            return 0;
        }

        1
    }
}
