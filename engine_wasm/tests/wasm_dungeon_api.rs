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
fn test_wasm_dungeon_generate() {
    let wat = r#"
        (module
            (import "dungeon" "generate_dungeon" (func $generate_dungeon (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            
            (data (i32.const 0) "{\"width\":40,\"height\":25,\"seed\":42}")
            
            (func (export "run_generate") (result i32)
                ;; config at offset 0, length of config string
                i32.const 0
                i32.const 38
                ;; output buffer at offset 256, max size 2048
                i32.const 256
                i32.const 2048
                call $generate_dungeon
                ;; Return the length of the result
            )
        )
    "#;
    let wasm_file = wat_to_tempfile(wat);

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };

    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("run_generate", &[])
        .expect("Failed to call run_generate");

    // Result is the length of the output string written to memory (should be > 0)
    let result_len = match result {
        Some(engine_wasm::WasmValue::I32(n)) => n,
        _ => panic!("Expected i32 result"),
    };
    assert!(result_len > 0, "Should return positive result length");
}

#[test]
fn test_wasm_dungeon_invalid_config() {
    let wat = r#"
        (module
            (import "dungeon" "generate_dungeon" (func $generate_dungeon (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            
            (data (i32.const 0) "{\"width\":0,\"height\":0}")
            
            (func (export "run_invalid") (result i32)
                i32.const 0
                i32.const 24
                i32.const 256
                i32.const 2048
                call $generate_dungeon
            )
        )
    "#;
    let wasm_file = wat_to_tempfile(wat);

    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
    };

    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("run_invalid", &[])
        .expect("Failed to call run_invalid");

    // Should still return a positive length (error message written to output)
    let result_len = match result {
        Some(engine_wasm::WasmValue::I32(n)) => n,
        _ => panic!("Expected i32 result"),
    };
    assert!(result_len > 0, "Should return error message length");
}
