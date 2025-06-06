use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig};
use std::io::Write;
use tempfile::NamedTempFile;

// Import the store type used by your engine
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::Caller;

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
    let wasm_bytes = load_wasm_test_artifact("test_entity_api.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_entity_api_bridge() {
    let wasm_file = compile_test_wasm();

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        import_host_functions: Some(Box::new(|linker| {
            // Explicit type annotation for the store
            type Store = Arc<Mutex<WasmWorld>>;

            linker
                .func_wrap("env", "spawn_entity", |_: Caller<'_, Store>| 42u32)
                .unwrap();
            linker
                .func_wrap(
                    "env",
                    "move_entity",
                    |_: Caller<'_, Store>, _: u32, _: f32, _: f32| {},
                )
                .unwrap();
            linker
                .func_wrap(
                    "env",
                    "damage_entity",
                    |_: Caller<'_, Store>, _: u32, _: f32| {},
                )
                .unwrap();
            linker
                .func_wrap("env", "is_entity_alive", |_: Caller<'_, Store>, _: u32| {
                    1i32
                })
                .unwrap();
            linker
                .func_wrap("env", "despawn_entity", |_: Caller<'_, Store>, _: u32| {})
                .unwrap();
        })),
    };

    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");

    // Call exported test function in WASM which will call host entity API and assert
    let result = engine
        .invoke_exported_function("test_entity_api", &[])
        .expect("Failed to call test_entity_api");
    assert_eq!(result, Some(1i32.into())); // Convention: return 1 for success
}
