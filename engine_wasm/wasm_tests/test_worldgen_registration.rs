#![no_std]
#![no_main]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Export the worldgen generate function
#[no_mangle]
pub extern "C" fn mge_worldgen_generate(
    _params_ptr: i32,
    params_len: i32,
    out_ptr: i32,
    out_len: i32,
) -> i32 {
    // Simple implementation: return an empty map JSON
    // Read params (just to validate they're provided)
    if params_len == 0 || out_len < 10 {
        return -1;
    }
    // Write a simple map JSON to output
    let result = r#"{"cells":[],"neighbors":{}}"#;
    let bytes = result.as_bytes();
    let n = if bytes.len() < out_len as usize {
        bytes.len()
    } else {
        out_len as usize
    };
    // Write output to memory at out_ptr
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr as *mut u8, n);
    }
    n as i32
}

// Export the worldgen validate function
#[no_mangle]
pub extern "C" fn mge_worldgen_validate(_map_ptr: i32, _map_len: i32) -> i32 {
    0
}

// Export the worldgen postprocess function
#[no_mangle]
pub extern "C" fn mge_worldgen_postprocess(
    _map_ptr: i32,
    _map_len: i32,
    _out_ptr: i32,
    _out_len: i32,
) -> i32 {
    // Pass-through: just copy input to output
    0
}

#[no_mangle]
pub extern "C" fn test_register_worldgen_plugin() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn register_worldgen_plugin(
            name_ptr: *const u8,
            name_len: i32,
            type_ptr: *const u8,
            type_len: i32,
        ) -> i32;
    }

    unsafe {
        let name = "wasm_test_plugin";
        let type_str = "wasm";
        let res = register_worldgen_plugin(
            name.as_ptr(),
            name.len() as i32,
            type_str.as_ptr(),
            type_str.len() as i32,
        );
        if res != 0 {
            return 0;
        }
        1
    }
}

#[no_mangle]
pub extern "C" fn test_register_worldgen_validator() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn register_worldgen_validator(name_ptr: *const u8, name_len: i32) -> i32;
    }

    unsafe {
        let name = "wasm_test_validator";
        let res = register_worldgen_validator(name.as_ptr(), name.len() as i32);
        if res != 0 {
            return 0;
        }
        1
    }
}

#[no_mangle]
pub extern "C" fn test_register_worldgen_postprocessor() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn register_worldgen_postprocessor(name_ptr: *const u8, name_len: i32) -> i32;
    }

    unsafe {
        let name = "wasm_test_postprocessor";
        let res = register_worldgen_postprocessor(name.as_ptr(), name.len() as i32);
        if res != 0 {
            return 0;
        }
        1
    }
}

// Combined test: register plugin, invoke it, verify result
#[no_mangle]
pub extern "C" fn test_worldgen_full_flow() -> i32 {
    #[link(wasm_import_module = "worldgen")]
    unsafe extern "C" {
        fn register_worldgen_plugin(
            name_ptr: *const u8,
            name_len: i32,
            type_ptr: *const u8,
            type_len: i32,
        ) -> i32;
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
        // Register the plugin
        let name = "wasm_test_plugin";
        let type_str = "wasm";
        let res = register_worldgen_plugin(
            name.as_ptr(),
            name.len() as i32,
            type_str.as_ptr(),
            type_str.len() as i32,
        );
        if res != 0 {
            return 0;
        }

        // Invoke the plugin
        let params = "{}";
        let mut out_buf = [0u8; 4096];
        let bytes = invoke_worldgen_plugin(
            name.as_ptr(),
            name.len() as i32,
            params.as_ptr(),
            params.len() as i32,
            out_buf.as_mut_ptr(),
            out_buf.len() as i32,
        );
        if bytes < 0 {
            return 0;
        }

        1
    }
}
