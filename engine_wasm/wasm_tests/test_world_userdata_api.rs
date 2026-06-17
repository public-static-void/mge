// This file is compiled to WASM and loaded by the Rust host test harness.
// It validates the world userdata host API (register_map_validator,
// clear_map_validators, register_map_postprocessor, clear_map_postprocessors,
// apply_chunk).

#![no_std]
#![no_main]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn mge_validate_map(_map_ptr: i32, _map_len: i32) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn mge_postprocess_map(_map_ptr: i32, _map_len: i32) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn test_world_userdata_api() -> i32 {
    #[link(wasm_import_module = "wasm_map")]
    unsafe extern "C" {
        fn register_map_validator(name_ptr: *const u8, name_len: i32) -> i32;
        fn clear_map_validators() -> i32;
        fn register_map_postprocessor(name_ptr: *const u8, name_len: i32) -> i32;
        fn clear_map_postprocessors() -> i32;
        fn apply_chunk(chunk_ptr: *const u8, chunk_len: i32) -> i32;
    }

    unsafe {
        // Register a map validator
        let validator_name = "mge_validate_map";
        let res = register_map_validator(validator_name.as_ptr(), validator_name.len() as i32);
        if res != 0 {
            return 0;
        }

        // Register a map postprocessor
        let postproc_name = "mge_postprocess_map";
        let res = register_map_postprocessor(postproc_name.as_ptr(), postproc_name.len() as i32);
        if res != 0 {
            return 0;
        }

        // Clear map validators
        let clear_res = clear_map_validators();
        if clear_res != 0 {
            return 0;
        }

        // Clear map postprocessors
        let clear_res = clear_map_postprocessors();
        if clear_res != 0 {
            return 0;
        }

        // Apply a chunk
        let chunk_json = r#"{"cells":[{"x":0,"y":0,"z":0}],"neighbors":[],"metadata":{}}"#;
        let res = apply_chunk(chunk_json.as_ptr(), chunk_json.len() as i32);
        if res != 0 {
            return 0;
        }

        // Re-register after clear to verify
        let res = register_map_validator(validator_name.as_ptr(), validator_name.len() as i32);
        if res != 0 {
            return 0;
        }

        1
    }
}
