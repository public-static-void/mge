//! WASM noise/detection parity tests.
//!
//! Proves the `wasm_noise` host surface through the real WASM linear-memory
//! JSON transport at Lua/Python parity (`test_noise.lua` / `test_noise.py`):
//! emit → tick → get round-trip values, hearing set/get with string-out,
//! and the missing-entity diagnostic channel.

use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig, WasmValue};
use std::io::Write;
use tempfile::NamedTempFile;

fn wat_to_tempfile(wat: &str) -> NamedTempFile {
    let wasm_bytes = wat::parse_str(wat).expect("Failed to parse WAT");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

fn engine_for(wat: &str) -> (NamedTempFile, WasmScriptEngine) {
    let wasm_file = wat_to_tempfile(wat);
    let config = WasmScriptEngineConfig {
        module_path: wasm_file.path().to_path_buf(),
        schema_path: None,
        worldgen_registry: None,
        import_host_functions: None,
        input_source: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    (wasm_file, engine)
}

fn assert_flag_one(engine: &WasmScriptEngine, export: &str) {
    let result = engine
        .invoke_exported_function(export, &[])
        .unwrap_or_else(|_| panic!("Failed to call {export}"));
    assert_eq!(result, Some(WasmValue::I32(1)), "{export} checks failed");
}

#[test]
fn test_wasm_noise_roundtrip() {
    let wat = r#"
        (module
            (import "entity" "spawn_entity" (func $spawn (result i32)))
            (import "component" "set_component" (func $set_comp (param i32 i32 i32 i32 i32)))
            (import "wasm_map" "add_cell" (func $add_cell (param i32 i32 i32)))
            (import "wasm_map" "add_neighbor" (func $add_neighbor (param i32 i32 i32 i32)))
            (import "wasm_noise" "emit_noise" (func $emit (param i32 f64 i32) (result i32)))
            (import "wasm_noise" "get_noise_at" (func $get_at (param i32 i32 i32) (result f64)))
            (import "turn" "tick" (func $tick))
            (memory (export "memory") 1)
            (data (i32.const 0) "Position")
            (data (i32.const 16) "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}")
            (data (i32.const 64) "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}")
            (data (i32.const 128) "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}")
            (data (i32.const 192) "{\"Square\":{\"x\":2,\"y\":0,\"z\":0}}")
            (func (export "test_noise_roundtrip") (result i32)
                (local $e i32)
                (local.set $e (call $spawn))
                (call $set_comp (local.get $e) (i32.const 0) (i32.const 8) (i32.const 16) (i32.const 38))
                (call $add_cell (i32.const 0) (i32.const 0) (i32.const 0))
                (call $add_cell (i32.const 1) (i32.const 0) (i32.const 0))
                (call $add_cell (i32.const 2) (i32.const 0) (i32.const 0))
                (call $add_cell (i32.const 6) (i32.const 0) (i32.const 0))
                (call $add_neighbor (i32.const 64) (i32.const 30) (i32.const 128) (i32.const 30))
                (call $add_neighbor (i32.const 128) (i32.const 30) (i32.const 192) (i32.const 30))
                (if (i32.ne (call $emit (local.get $e) (f64.const 1.0) (i32.const 5)) (i32.const 1))
                    (then (return (i32.const 0))))
                (call $tick)
                (if (i32.eqz (f64.eq (call $get_at (i32.const 0) (i32.const 0) (i32.const 0)) (f64.const 1.0)))
                    (then (return (i32.const 0))))
                (if (i32.eqz (f64.eq (call $get_at (i32.const 1) (i32.const 0) (i32.const 0)) (f64.const 0.8)))
                    (then (return (i32.const 0))))
                (if (i32.eqz (f64.eq (call $get_at (i32.const 2) (i32.const 0) (i32.const 0)) (f64.const 0.6)))
                    (then (return (i32.const 0))))
                (if (i32.eqz (f64.eq (call $get_at (i32.const 6) (i32.const 0) (i32.const 0)) (f64.const 0.0)))
                    (then (return (i32.const 0))))
                (i32.const 1)
            )
        )
    "#;
    let (_file, engine) = engine_for(wat);
    assert_flag_one(&engine, "test_noise_roundtrip");
}

#[test]
fn test_wasm_noise_hearing() {
    let wat = r#"
        (module
            (import "entity" "spawn_entity" (func $spawn (result i32)))
            (import "wasm_noise" "set_hearing" (func $set_h (param i32 i32 f64) (result i32)))
            (import "wasm_noise" "get_hearing" (func $get_h (param i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 512) "\"range\"")
            (func $match7 (param $off i32) (result i32)
                (i32.and
                    (i32.and
                        (i32.and
                            (i32.eq (i32.load8_u (local.get $off)) (i32.load8_u (i32.const 512)))
                            (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 1))) (i32.load8_u (i32.const 513))))
                        (i32.and
                            (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 2))) (i32.load8_u (i32.const 514)))
                            (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 3))) (i32.load8_u (i32.const 515)))))
                    (i32.and
                        (i32.and
                            (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 4))) (i32.load8_u (i32.const 516)))
                            (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 5))) (i32.load8_u (i32.const 517))))
                        (i32.eq (i32.load8_u (i32.add (local.get $off) (i32.const 6))) (i32.load8_u (i32.const 518))))))
            (func (export "test_noise_hearing") (result i32)
                (local $e i32) (local $n i32) (local $i i32) (local $found i32)
                (local.set $e (call $spawn))
                (if (i32.ne (call $get_h (local.get $e) (i32.const 0) (i32.const 256)) (i32.const -1))
                    (then (return (i32.const 0))))
                (if (i32.ne (call $set_h (local.get $e) (i32.const 5) (f64.const 1.0)) (i32.const 1))
                    (then (return (i32.const 0))))
                (local.set $n (call $get_h (local.get $e) (i32.const 0) (i32.const 256)))
                (if (i32.le_s (local.get $n) (i32.const 0))
                    (then (return (i32.const 0))))
                (local.set $i (i32.const 0))
                (local.set $found (i32.const 0))
                (block $done
                    (loop $scan
                        (br_if $done (i32.gt_u (local.get $i) (i32.sub (local.get $n) (i32.const 7))))
                        (if (call $match7 (local.get $i))
                            (then (local.set $found (i32.const 1)) (br $done)))
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        (br $scan)))
                (if (i32.eqz (local.get $found))
                    (then (return (i32.const 0))))
                (i32.const 1)
            )
        )
    "#;
    let (_file, engine) = engine_for(wat);
    assert_flag_one(&engine, "test_noise_hearing");
}

#[test]
fn test_wasm_noise_missing_entity() {
    let wat = r#"
        (module
            (import "wasm_noise" "get_hearing" (func $get_h (param i32 i32 i32) (result i32)))
            (import "wasm_noise" "get_noise_at" (func $get_at (param i32 i32 i32) (result f64)))
            (memory (export "memory") 1)
            (func (export "test_noise_missing_entity") (result i32)
                (if (i32.ne (call $get_h (i32.const 999999) (i32.const 0) (i32.const 256)) (i32.const -1))
                    (then (return (i32.const 0))))
                (if (i32.eqz (f64.eq (call $get_at (i32.const 50) (i32.const 60) (i32.const 70)) (f64.const 0.0)))
                    (then (return (i32.const 0))))
                (i32.const 1)
            )
        )
    "#;
    let (_file, engine) = engine_for(wat);
    assert_flag_one(&engine, "test_noise_missing_entity");
}
