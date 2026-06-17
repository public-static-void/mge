use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig, WasmValue};
use std::io::Write;
use tempfile::NamedTempFile;

fn load_wasm_test_artifact(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("wasm_tests")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "Failed to load WASM test artifact '{}': {}",
            path.display(),
            e
        )
    })
}

fn compile_test_wasm() -> NamedTempFile {
    let wasm_bytes = load_wasm_test_artifact("test_economic_reservation.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_reserve_only() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_reserve_job_resources_only", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_get_reservations_before() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_get_job_resource_reservations_before", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_reserve_then_query() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_reserve_then_query", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_reserve_and_release() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_reserve_and_release", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}
