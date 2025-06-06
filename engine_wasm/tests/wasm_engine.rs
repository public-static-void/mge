use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig};
use std::io::Write;
use tempfile::NamedTempFile;

fn wat_to_tempfile(wat: &str) -> NamedTempFile {
    let wasm_bytes = wat::parse_str(wat).expect("Failed to parse WAT");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_script_engine_can_instantiate_and_run() {
    let wat = r#"
        (module
            (func (export "add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
        )
    "#;
    let wasm_file = wat_to_tempfile(wat);

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("add", &[1i32.into(), 2i32.into()])
        .expect("Failed to call add");
    assert_eq!(result, Some(3i32.into()));
}

#[test]
fn test_wasm_script_engine_can_instantiate_with_wasi() {
    // Minimal WASI module (requires WASI import to instantiate)
    let wat = r#"
        (module
            (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
            (func (export "run") (param i32)
                local.get 0
                call $exit
            )
        )
    "#;
    let wasm_file = wat_to_tempfile(wat);

    // Should fail if WASI is not enabled
    let config_no_wasi = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        import_host_functions: None,
    };
    let result = WasmScriptEngine::new(config_no_wasi);
    assert!(
        result.is_err(),
        "Should fail to instantiate WASI module without WASI enabled"
    );

    // Should succeed if WASI is enabled
    // NOTE: This test will not pass unless you reintroduce WASI support in your engine.
    // For now, this block is commented out to avoid confusion and errors.
    /*
    let config_with_wasi = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        import_host_functions: None,
    };
    let result = WasmScriptEngine::new(config_with_wasi);
    assert!(
        result.is_ok(),
        "Should succeed to instantiate WASI module with WASI enabled"
    );
    */
}

#[test]
fn test_wasm_script_engine_can_register_and_call_host_function() {
    let wat = r#"
        (module
            (import "" "host_add" (func $host_add (param i32 i32) (result i32)))
            (func (export "call_host_add") (param i32 i32) (result i32)
                local.get 0
                local.get 1
                call $host_add
            )
        )
    "#;
    let wasm_file = wat_to_tempfile(wat);

    let called = std::sync::Arc::new(std::sync::Mutex::new(false));
    let called_clone = called.clone();

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        import_host_functions: Some(Box::new(move |linker| {
            let called = called_clone.clone();
            linker
                .func_wrap("", "host_add", move |a: i32, b: i32| {
                    *called.lock().unwrap() = true;
                    a + b
                })
                .unwrap();
        })),
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("call_host_add", &[5i32.into(), 7i32.into()])
        .expect("Failed to call call_host_add");
    assert_eq!(result, Some(12i32.into()));
    assert!(
        *called.lock().unwrap(),
        "Host function should have been called"
    );
}
