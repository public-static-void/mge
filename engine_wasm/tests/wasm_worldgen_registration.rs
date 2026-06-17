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
    let wasm_bytes = load_wasm_test_artifact("test_worldgen_registration.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_register_worldgen_plugin() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_register_worldgen_plugin", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_register_worldgen_validator() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_register_worldgen_validator", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_register_worldgen_postprocessor() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_register_worldgen_postprocessor", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

#[test]
fn test_wasm_worldgen_full_flow() {
    let wasm_file = compile_test_wasm();
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_worldgen_full_flow", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}
