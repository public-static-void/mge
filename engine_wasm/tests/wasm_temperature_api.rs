use engine_core::ecs::world::wasm::WasmWorld;
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
    let wasm_bytes = load_wasm_test_artifact("test_temperature_api.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

#[test]
fn test_wasm_temperature_api_bridge() {
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
        .invoke_exported_function("test_temperature_api", &[])
        .expect("Failed to call test_temperature_api");
    assert_eq!(result, Some(1i32.into()));
}

#[test]
fn test_wasm_temperature_tick_holds_override() {
    let mut world = WasmWorld::new();
    assert!((world.get_temperature() - 15.0).abs() < 0.0001);

    world.set_temperature(20.0);
    assert!((world.get_temperature() - 20.0).abs() < 0.0001);

    world.tick();
    assert!((world.get_temperature() - 20.0).abs() < 0.0001);

    world.set_temperature(100.0);
    assert!((world.get_temperature() - 60.0).abs() < 0.0001);

    world.set_temperature(-100.0);
    assert!((world.get_temperature() + 60.0).abs() < 0.0001);

    world.tick();
    assert!((world.get_temperature() + 60.0).abs() < 0.0001);
}

#[test]
fn test_wasm_temperature_emits_changed_event() {
    let mut world = WasmWorld::new();
    world.set_temperature(20.0);
    let events_json = world.take_events("temperature_changed");
    let events: Vec<serde_json::Value> =
        serde_json::from_str(&events_json).expect("temperature_changed events parse as JSON");
    assert_eq!(events.len(), 1);
    let old_ambient = events[0]["old_ambient"].as_f64().unwrap();
    let new_ambient = events[0]["new_ambient"].as_f64().unwrap();
    assert!((old_ambient - 15.0).abs() < 0.0001);
    assert!((new_ambient - 20.0).abs() < 0.0001);
}

#[test]
fn test_wasm_temperature_tick_derives_without_override() {
    let mut world = WasmWorld::new();
    world.tick();
    let season = engine_core::ecs::world::Season::from_day(world.time_of_day.day);
    let expected = engine_core::systems::temperature::compute_ambient_temperature(
        season,
        world.weather.condition,
        world.weather.intensity,
        world.time_of_day.hour,
        world.time_of_day.minute,
        world.weather.humidity,
        world.weather.pressure,
    );
    assert!((world.get_temperature() - expected).abs() < 0.0001);
}
