use engine_core::systems::cellular_caves::{CellularCavesConfig, CellularCavesGenerator};
use engine_core::systems::dungeon::DungeonGenerator;
use engine_core::worldgen::{
    MapgenAlgoEntry, MapgenAlgorithm, ThreadSafeWorldgenPlugin, ThreadSafeWorldgenRegistry,
    WorldgenPlugin, WorldgenRegistry, register_builtin_mapgen_algorithms,
    register_builtin_mapgen_algorithms_local, resolve_algorithm_name,
};
use serde_json::json;

fn switch_params() -> serde_json::Value {
    json!({"seed": 42, "width": 20, "height": 15})
}

#[test]
fn both_algorithms_register_under_distinct_names() {
    assert_eq!(DungeonGenerator.name(), "dungeon");
    assert_eq!(CellularCavesGenerator.name(), "caves");
    assert_ne!(DungeonGenerator.name(), CellularCavesGenerator.name());
}

#[test]
fn switching_algorithms_is_a_name_string_change() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let params = switch_params();
    let dungeon_map = registry
        .invoke("dungeon", &params)
        .expect("dungeon invokes");
    let caves_map = registry.invoke("caves", &params).expect("caves invokes");

    assert_eq!(dungeon_map["topology"], "square");
    assert_eq!(caves_map["topology"], "square");
    assert!(!dungeon_map["cells"].as_array().unwrap().is_empty());
    assert!(!caves_map["cells"].as_array().unwrap().is_empty());
}

#[test]
fn caves_output_is_deterministic_for_a_fixed_seed() {
    let algo = CellularCavesGenerator;
    let first = algo
        .generate(&json!({"seed": 7, "width": 20, "height": 15}))
        .expect("generates");
    let second = algo
        .generate(&json!({"seed": 7, "width": 20, "height": 15}))
        .expect("generates");
    let first_str = serde_json::to_string(&first).unwrap();
    let second_str = serde_json::to_string(&second).unwrap();
    assert_eq!(first_str, second_str);
}

#[test]
fn caves_output_differs_across_seeds() {
    let algo = CellularCavesGenerator;
    let seed_a = algo
        .generate(&json!({"seed": 42, "width": 20, "height": 15}))
        .expect("generates");
    let seed_b = algo
        .generate(&json!({"seed": 99, "width": 20, "height": 15}))
        .expect("generates");
    assert_ne!(
        serde_json::to_string(&seed_a).unwrap(),
        serde_json::to_string(&seed_b).unwrap()
    );
}

#[test]
fn dungeon_output_is_deterministic_for_a_fixed_seed() {
    let algo = DungeonGenerator;
    let first = algo.generate(&switch_params()).expect("generates");
    let second = algo.generate(&switch_params()).expect("generates");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}

#[test]
fn params_algorithm_field_overrides_config_default() {
    let from_params = resolve_algorithm_name(Some("dungeon"), &json!({"algorithm": "caves"}));
    assert_eq!(from_params, "caves");

    let from_config = resolve_algorithm_name(Some("caves"), &json!({}));
    assert_eq!(from_config, "caves");

    let fallback = resolve_algorithm_name(None, &json!({}));
    assert_eq!(fallback, "dungeon");
}

#[test]
fn unknown_name_error_lists_both_algorithms() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let err = registry
        .invoke("no_such_algorithm", &switch_params())
        .expect_err("unknown name must fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("dungeon"),
        "candidates list dungeon: {message}"
    );
    assert!(
        message.contains("caves"),
        "candidates list caves: {message}"
    );
}

#[test]
fn local_registry_invokes_the_new_algorithm() {
    let mut registry = WorldgenRegistry::new();
    register_builtin_mapgen_algorithms_local(&mut registry);

    let names = registry.names();
    assert!(names.contains(&"dungeon".to_string()));
    assert!(names.contains(&"caves".to_string()));

    let caves_map = registry
        .invoke("caves", &switch_params())
        .expect("local caves invoke");
    assert_eq!(caves_map["topology"], "square");
}

#[test]
fn builtin_registration_is_idempotent() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);
    register_builtin_mapgen_algorithms(&mut registry);
    assert_eq!(registry.names().len(), 2);
}

#[test]
fn algorithm_entries_adapt_into_both_plugin_enums() {
    let threadsafe: ThreadSafeWorldgenPlugin = MapgenAlgoEntry(CellularCavesGenerator).into();
    assert_eq!(threadsafe.name(), "caves");

    let local: WorldgenPlugin = MapgenAlgoEntry(CellularCavesGenerator).into();
    assert_eq!(local.name(), "caves");
}

#[test]
fn caves_config_carries_an_explicit_seed() {
    let config = CellularCavesConfig {
        seed: 1234,
        ..CellularCavesConfig::explicit(20, 15, 1234)
    };
    assert_eq!(config.seed, 1234);
    assert_eq!(config.width, 20);
    assert_eq!(config.height, 15);
}
