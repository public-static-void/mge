/// Standing suite for the event-driven narrative engine (incident director).
///
/// Drives the identical seven-function surface through the WASM bridge with
/// a guest test artifact: register/list/get, poll/get-pending, resolve,
/// history. Trigger evaluation stays in Rust core; the guest only
/// registers, ticks, polls, and resolves.
use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig};
use std::io::Write;
use tempfile::NamedTempFile;

/// Loads a WASM test artifact from the wasm_tests directory at runtime.
/// Panics if the file is missing.
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

/// Writes the loaded WASM bytes to a temporary file and returns the file handle.
fn compile_test_wasm() -> NamedTempFile {
    let wasm_bytes = load_wasm_test_artifact("test_narrative_api.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn narrative_content_file_loads_without_breaking_startup() {
    let defs = engine_core::narrative::load_scenario_definitions();
    assert!(
        defs.iter().all(|d| !d.id.is_empty()),
        "loaded scenario definitions must carry non-empty ids"
    );
}

#[test]
fn test_wasm_narrative_api_bridge() {
    let wasm_file = compile_test_wasm();

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
        input_source: None,
    };

    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");

    let result = engine
        .invoke_exported_function("test_narrative_api", &[])
        .expect("Failed to call test_narrative_api");
    assert_eq!(result, Some(1i32.into()));
}
