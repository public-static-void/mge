// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_component_introspection_api() -> i32 {
    #[link(wasm_import_module = "component")]
    unsafe extern "C" {
        fn list_components(out_ptr: *mut u8, out_len: i32) -> i32;
        fn get_component_schema(
            name_ptr: *const u8,
            name_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        // Test list_components
        let mut buf1 = [0u8; 4096];
        let written = list_components(buf1.as_mut_ptr(), buf1.len() as i32);
        if written < 0 {
            return 0;
        }

        // Test get_component_schema with "Position"
        let name = "Position";
        let mut buf2 = [0u8; 4096];
        let written2 = get_component_schema(
            name.as_ptr(),
            name.len() as i32,
            buf2.as_mut_ptr(),
            buf2.len() as i32,
        );
        if written2 < 0 {
            return 0;
        }

        // Test get_component_schema with nonexistent name (should return -1)
        let bad_name = "___nonexistent___";
        let mut buf3 = [0u8; 128];
        let written3 = get_component_schema(
            bad_name.as_ptr(),
            bad_name.len() as i32,
            buf3.as_mut_ptr(),
            buf3.len() as i32,
        );
        if written3 != -1 {
            return 0;
        }

        1
    }
}
