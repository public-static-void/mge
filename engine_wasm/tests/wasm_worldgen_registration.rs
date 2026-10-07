use engine_core::worldgen::{
    GuestFailureStage, ThreadSafeScriptingWorldgenPlugin, ThreadSafeWorldgenPlugin,
    ThreadSafeWorldgenRegistry, WorldgenError,
};
use engine_wasm::host_api::worldgen::{WasmGuestWorldgenPlugin, wasm_layout};
use engine_wasm::{WasmScriptEngine, WasmScriptEngineConfig, WasmValue};
use serde_json::json;
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
        input_source: None,
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
        input_source: None,
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
        input_source: None,
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
        input_source: None,
    };
    let engine = WasmScriptEngine::new(config).expect("Failed to create WasmScriptEngine");
    let result = engine
        .invoke_exported_function("test_worldgen_full_flow", &[])
        .expect("Failed to call function");
    assert_eq!(result, Some(WasmValue::I32(1)));
}

fn guest_entry(name: &str) -> ThreadSafeWorldgenPlugin {
    WasmGuestWorldgenPlugin::host_routed(name).into_registry_entry()
}

#[test]
fn guest_adapter_registers_as_real_registry_entry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    let adapter = WasmGuestWorldgenPlugin::host_routed("guest_caves");
    assert_eq!(adapter.name(), "guest_caves");
    assert_eq!(adapter.backend(), WasmGuestWorldgenPlugin::BACKEND);

    registry
        .register(adapter.into_registry_entry())
        .expect("guest entry registers");
    assert!(registry.names().contains(&"guest_caves".to_string()));

    let duplicate = registry.register(guest_entry("guest_caves"));
    assert!(duplicate.is_err());

    assert!(registry.unregister("guest_caves"));
    assert!(!registry.names().contains(&"guest_caves".to_string()));
    assert!(!registry.unregister("guest_caves"));
}

#[test]
fn guest_missing_export_reports_no_export_stage() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    let broken = WasmGuestWorldgenPlugin::with_invoker("broken_guest", |_| {
        Err(WorldgenError::Guest {
            stage: GuestFailureStage::NoExport,
            detail: "missing export mge_worldgen_generate".to_string(),
        })
    });
    registry
        .register(broken.into_registry_entry())
        .expect("guest entry registers");

    let err = registry.invoke("broken_guest", &json!({})).unwrap_err();
    match err {
        WorldgenError::Guest { stage, .. } => assert_eq!(stage, GuestFailureStage::NoExport),
        other => panic!("expected Guest diagnostic, got {other:?}"),
    }
}

#[test]
fn guest_unreadable_memory_reports_no_memory_stage() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    let broken = WasmGuestWorldgenPlugin::with_invoker("dark_guest", |_| {
        Err(WorldgenError::Guest {
            stage: GuestFailureStage::NoMemory,
            detail: "guest linear memory unavailable".to_string(),
        })
    });
    registry
        .register(broken.into_registry_entry())
        .expect("guest entry registers");

    let err = registry.invoke("dark_guest", &json!({})).unwrap_err();
    match err {
        WorldgenError::Guest { stage, .. } => assert_eq!(stage, GuestFailureStage::NoMemory),
        other => panic!("expected Guest diagnostic, got {other:?}"),
    }
}

#[test]
fn guest_host_routed_entry_explains_registry_invoke_path() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(guest_entry("routed_guest"))
        .expect("guest entry registers");

    let err = registry.invoke("routed_guest", &json!({})).unwrap_err();
    match err {
        WorldgenError::Guest { stage, detail } => {
            assert_eq!(stage, GuestFailureStage::RegistryInvoke);
            assert!(!detail.is_empty());
        }
        other => panic!("expected Guest diagnostic, got {other:?}"),
    }
}

#[test]
fn guest_stage_codes_replace_silent_minus_one() {
    let stages = [
        GuestFailureStage::NoExport,
        GuestFailureStage::NoMemory,
        GuestFailureStage::WriteParams,
        GuestFailureStage::Call,
        GuestFailureStage::ReadResult,
        GuestFailureStage::ParseResult,
        GuestFailureStage::Validate,
        GuestFailureStage::Postprocess,
        GuestFailureStage::RegistryInvoke,
    ];
    let codes: Vec<i32> = stages.iter().map(GuestFailureStage::code).collect();
    for code in &codes {
        assert!(*code != -1 && *code != 0);
    }
    let mut unique = codes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), stages.len());

    let registry = ThreadSafeWorldgenRegistry::new();
    let err = registry.invoke("no_such_plugin", &json!({})).unwrap_err();
    assert!(matches!(err, WorldgenError::NotFound { .. }));
    assert_eq!(err.guest_code(), -1);
}

#[test]
fn guest_scratch_sizes_are_named_constants() {
    assert_eq!(wasm_layout::PARAM_SCRATCH_OFFSET, 1024);
    assert_eq!(wasm_layout::RESULT_SCRATCH_OFFSET, 4096);
    assert_eq!(wasm_layout::RESULT_SCRATCH_MAX, 4096);
}
