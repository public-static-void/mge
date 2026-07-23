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
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
        input_source: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("add", &[1i32.into(), 2i32.into()])
        .expect("Failed to call add");
    assert_eq!(result, Some(3i32.into()));
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
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: Some(Box::new(move |linker| {
            let called = called_clone.clone();
            linker
                .func_wrap("", "host_add", move |a: i32, b: i32| {
                    *called.lock().unwrap() = true;
                    a + b
                })
                .unwrap();
        })),
        input_source: None,
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
