// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_component_api() -> i32 {
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
        fn remove_component(entity: u32, name_ptr: *const u8, name_len: i32);
    }

    unsafe {
        let eid = spawn_entity();

        // Set a Position component, similar to the Lua test pattern
        let comp_name = "Position";
        let json_data = "{\"x\":3.0,\"y\":4.0}";
        set_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            json_data.as_ptr(),
            json_data.len() as i32,
        );

        // Get the component back and verify
        let mut out_buf = [0u8; 128];
        let written = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if written < 0 {
            return 0;
        }
        let written = written as usize;

        // Verify the content matches
        let expected = json_data.as_bytes();
        if written != expected.len() {
            return 0;
        }
        for i in 0..written {
            if out_buf[i] != expected[i] {
                return 0;
            }
        }

        // Remove the component
        remove_component(eid, comp_name.as_ptr(), comp_name.len() as i32);

        // Verify it's gone
        let mut out_buf2 = [0u8; 128];
        let written2 = get_component(
            eid,
            comp_name.as_ptr(),
            comp_name.len() as i32,
            out_buf2.as_mut_ptr(),
            out_buf2.len() as i32,
        );
        if written2 != -1 {
            return 0;
        }

        1
    }
}
