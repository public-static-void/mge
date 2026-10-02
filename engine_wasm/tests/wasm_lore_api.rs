/// Standing suite for procedural history and lore on the WASM bridge.
///
/// Drives the identical six-function surface (`generate_founding_history`,
/// `list_chronicle`, `get_chronicle_entry`, `render_chronicle`,
/// `chronicle_len`, `clear_lore_history`) through a guest test artifact,
/// plus host-side `WasmWorld` mirror coverage: deterministic backfill,
/// same-tick narrative feed, and snapshot/restore round-trips.
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::lore::ChronicleFilter;
use engine_core::narrative::{ScenarioChoice, ScenarioDef, TriggerPredicate};
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
    let wasm_bytes = load_wasm_test_artifact("test_lore_api.wasm");
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&wasm_bytes)
        .expect("Failed to write WASM module");
    file
}

/// Builds a minimal firing scenario for host-side feed tests.
fn probe_scenario(id: &str) -> ScenarioDef {
    ScenarioDef {
        id: id.to_string(),
        name: "Probe".to_string(),
        description: String::new(),
        triggers: vec![TriggerPredicate::TurnGte { turn: 0 }],
        choices: vec![ScenarioChoice {
            id: "take".to_string(),
            label: "Take".to_string(),
            effects: vec![],
        }],
        cooldown_turns: 0,
        expires_in_turns: None,
        once: false,
        weight: 1.0,
    }
}

#[test]
fn test_wasm_lore_api_bridge() {
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
        .invoke_exported_function("test_lore_api", &[])
        .expect("Failed to call test_lore_api");
    assert_eq!(result, Some(1i32.into()));
}

#[test]
fn backfill_is_deterministic_across_worlds() {
    let mut first = WasmWorld::new();
    let mut second = WasmWorld::new();
    assert_eq!(
        first
            .generate_founding_history(42, 5)
            .expect("backfill succeeds"),
        5
    );
    assert_eq!(
        second
            .generate_founding_history(42, 5)
            .expect("backfill succeeds"),
        5
    );
    let all = ChronicleFilter::default();
    assert_eq!(
        first.list_chronicle(all.clone()),
        second.list_chronicle(all)
    );
}

#[test]
fn backfill_zero_count_appends_nothing() {
    let mut world = WasmWorld::new();
    assert_eq!(
        world
            .generate_founding_history(7, 0)
            .expect("zero backfill succeeds"),
        0
    );
    assert_eq!(world.chronicle_len(), 0);
}

#[test]
fn fired_records_are_visible_in_the_same_tick() {
    let mut world = WasmWorld::new();
    world
        .narrative
        .register_scenario(probe_scenario("host_probe"))
        .expect("scenario registers");
    world.tick_narrative();
    let fired = world.list_chronicle(ChronicleFilter {
        kind: Some(engine_core::lore::ChronicleKind::Fired),
        ..Default::default()
    });
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].scenario_id, "host_probe");
}

#[test]
fn resolved_records_mirror_without_an_extra_tick() {
    let mut world = WasmWorld::new();
    world
        .narrative
        .register_scenario(probe_scenario("host_probe"))
        .expect("scenario registers");
    world.tick_narrative();
    world
        .resolve_narrative_decision(0, "take")
        .expect("decision resolves");
    let resolved = world.list_chronicle(ChronicleFilter {
        kind: Some(engine_core::lore::ChronicleKind::Resolved),
        ..Default::default()
    });
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].choice_id.as_deref(), Some("take"));
}

#[test]
fn lore_snapshot_restores_entries_and_keeps_ids_monotonic() {
    let mut world = WasmWorld::new();
    world
        .generate_founding_history(42, 3)
        .expect("backfill succeeds");
    let json = serde_json::to_string(&world).expect("WasmWorld serializes");
    let mut loaded: WasmWorld = serde_json::from_str(&json).expect("snapshot reloads");
    assert_eq!(
        loaded.list_chronicle(ChronicleFilter::default()),
        world.list_chronicle(ChronicleFilter::default())
    );
    assert_eq!(loaded.lore.next_entry_id, world.lore.next_entry_id);
    loaded
        .generate_founding_history(9, 2)
        .expect("backfill succeeds after restore");
    let ids: Vec<u64> = loaded
        .list_chronicle(ChronicleFilter::default())
        .iter()
        .map(|entry| entry.entry_id)
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "entry ids stay unique");
    assert!(
        ids.iter().max().copied().unwrap_or(0) >= 3,
        "ids continue past the restored counter"
    );
}

#[test]
fn saves_without_lore_fields_load_with_an_empty_chronicle() {
    let mut value = serde_json::to_value(WasmWorld::new()).expect("fresh world serializes");
    value
        .as_object_mut()
        .expect("world serializes as an object")
        .remove("lore");
    let loaded: WasmWorld = serde_json::from_value(value).expect("old save loads");
    assert_eq!(loaded.chronicle_len(), 0);
    assert!(loaded.get_chronicle_entry(0).is_none());
}
