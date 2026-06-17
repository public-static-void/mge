// This file is compiled to WASM and loaded by the Rust host test harness.
#[no_mangle]
pub extern "C" fn test_worldgen_api() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn list_worldgen_plugins(out_ptr: *mut u8, out_len: i32) -> i32;
    }

    unsafe {
        // list_worldgen_plugins should return a JSON array (empty or with plugins)
        let mut buf = [0u8; 4096];
        let written = list_worldgen_plugins(buf.as_mut_ptr(), buf.len() as i32);
        if written < 0 {
            return 0;
        }

        // Just having no crash and a non-negative return is success
        1
    }
}
