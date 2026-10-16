use engine_core::worldgen::{ThreadSafeWorldgenRegistry, register_builtin_mapgen_algorithms};
use serde_json::json;

fn switch_params() -> serde_json::Value {
    json!({"seed": 42, "width": 20, "height": 15})
}

fn planet_biome_params(seed: i64) -> serde_json::Value {
    json!({"seed": seed, "width": 20, "height": 15})
}

fn assert_biome_shape(map: &serde_json::Value) {
    assert_eq!(map["topology"], "square");
    let cells = map["cells"]
        .as_array()
        .expect("planet_biomes map has cells");
    assert!(!cells.is_empty());
    for cell in cells {
        let biome = cell["biome"].as_str().unwrap_or("");
        assert!(
            !biome.is_empty(),
            "every cell carries a non-empty biome label"
        );
    }
}

#[test]
fn wasm_registry_path_invokes_both_algorithms_by_name() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let names = registry.names();
    assert!(names.contains(&"dungeon".to_string()));
    assert!(names.contains(&"caves".to_string()));

    let params = switch_params();
    let dungeon_map = registry
        .invoke("dungeon", &params)
        .expect("dungeon invokes");
    let caves_map = registry.invoke("caves", &params).expect("caves invokes");
    assert_eq!(dungeon_map["topology"], "square");
    assert_eq!(caves_map["topology"], "square");
}

#[test]
fn wasm_registry_path_caves_output_is_deterministic() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let params = json!({"seed": 7, "width": 20, "height": 15});
    let first = registry.invoke("caves", &params).expect("generates");
    let second = registry.invoke("caves", &params).expect("generates");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );

    let other = registry
        .invoke("caves", &json!({"seed": 99, "width": 20, "height": 15}))
        .expect("generates");
    assert_ne!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&other).unwrap()
    );
}

#[test]
fn wasm_registry_path_lists_planet_biomes() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let names = registry.names();
    assert!(names.contains(&"planet_biomes".to_string()));

    let planet_map = registry
        .invoke("planet_biomes", &planet_biome_params(42))
        .expect("planet_biomes invokes");
    assert_biome_shape(&planet_map);
}

#[test]
fn wasm_registry_path_planet_biomes_output_is_deterministic() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let first = registry
        .invoke("planet_biomes", &planet_biome_params(7))
        .expect("generates");
    let second = registry
        .invoke("planet_biomes", &planet_biome_params(7))
        .expect("generates");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );

    let seed_a = registry
        .invoke("planet_biomes", &planet_biome_params(42))
        .expect("generates");
    let seed_b = registry
        .invoke("planet_biomes", &planet_biome_params(99))
        .expect("generates");
    assert_ne!(
        serde_json::to_string(&seed_a).unwrap(),
        serde_json::to_string(&seed_b).unwrap()
    );
}

#[test]
fn wasm_registry_path_planet_biomes_default_size_succeeds() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let defaulted = registry
        .invoke("planet_biomes", &json!({"seed": 42}))
        .expect("planet_biomes default params invoke");
    assert_biome_shape(&defaulted);
    assert_eq!(defaulted["cells"].as_array().expect("cells").len(), 32 * 32);
}
