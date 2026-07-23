use engine_wasm::engine::EXPORT_WORLDGEN_GENERATE;
use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig};
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
    let wasm_bytes = load_wasm_test_artifact("test_export_discovery.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_export_discovery() {
    let wasm_file = compile_test_wasm();

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
        input_source: None,
    };

    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");

    // Verify the known export was discovered
    let result = engine
        .invoke_exported_function("test_export_discovery", &[])
        .expect("Failed to call test_export_discovery");
    assert_eq!(result, Some(1i32.into()));

    // Verify call_export works for discovered exports
    // mge_worldgen_generate takes (params_ptr: i32, params_len: i32) -> i32
    let result = engine
        .call_export(
            EXPORT_WORLDGEN_GENERATE,
            &[wasmtime::Val::I32(0), wasmtime::Val::I32(0)],
        )
        .expect("Failed to call mge_worldgen_generate via call_export");
    assert!(
        matches!(result, wasmtime::Val::I32(1)),
        "Expected I32(1), got {:?}",
        result
    );
}
