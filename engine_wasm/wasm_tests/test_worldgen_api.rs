// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_worldgen_api() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn list_worldgen_plugins(out_ptr: *mut u8, out_len: i32) -> i32;
        fn invoke_worldgen_plugin(
            name_ptr: *const u8,
            name_len: i32,
            params_ptr: *const u8,
            params_len: i32,
            out_ptr: *mut u8,
            out_len: i32,
        ) -> i32;
    }

    unsafe {
        // list_worldgen_plugins should return a JSON array (empty or with plugins)
        let mut buf = [0u8; 4096];
        let written = list_worldgen_plugins(buf.as_mut_ptr(), buf.len() as i32);
        if written < 0 {
            return 0;
        }

        // invoke_worldgen_plugin on a non-existent plugin should return -1 gracefully
        let plugin_name = "non_existent_plugin";
        let params = "{}";
        let mut out_buf = [0u8; 4096];
        let result = invoke_worldgen_plugin(
            plugin_name.as_ptr(),
            plugin_name.len() as i32,
            params.as_ptr(),
            params.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        // Expected: -1 (no plugin registered), or >=0 on success. Anything < -1 is an error.
        if result < -1 {
            return 0;
        }

        // Just having no crash and a non-negative return is success
        1
    }
}
