use engine_core::ecs::world::wasm::{WasmWorld, load_schemas_from_dir};

fn make_world() -> WasmWorld {
    let mut world = WasmWorld::new();
    let schema_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("engine")
        .join("assets")
        .join("schemas");
    if schema_dir.exists() {
        let schemas = load_schemas_from_dir(&schema_dir);
        // Key by schema title (matching the engine loader's name > title >
        // filename priority); the WasmWorld loader falls back to the lowercase
        // file stem, which would miss component-name lookups.
        world.component_schemas = schemas
            .into_iter()
            .map(|(name, schema)| {
                let key = schema
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .unwrap_or(name);
                (key, schema)
            })
            .collect();
    }
    world
}

fn species_json(diet: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "diet": diet,
        "graze_nutrition_rate": 0.2,
        "metabolism_rate": 0.05,
        "reproduction_threshold": 0.8,
        "reproduction_cooldown_ticks": 10,
        "litter_size": 1,
        "detection_range": 6,
        "noise_flee_threshold": 0.5,
        "activity": "nocturnal",
    }))
    .unwrap()
}

#[test]
fn test_wasm_ecosystem_schemas_registered_with_modes_and_defaults() {
    let world = make_world();
    for name in ["Species", "Wildlife"] {
        let schema = world
            .get_component_schema(name)
            .unwrap_or_else(|| panic!("{name} schema must be registered"));
        let parsed: serde_json::Value =
            serde_json::from_str(&schema).expect("schema parses as JSON");
        let modes = parsed["modes"].as_array().expect("modes must be an array");
        for mode in ["colony", "roguelike", "simulation"] {
            assert!(
                modes.iter().any(|m| m.as_str() == Some(mode)),
                "{name} schema must allow {mode}"
            );
        }
        let properties = parsed["properties"]
            .as_object()
            .expect("schema must declare properties");
        assert!(!properties.is_empty(), "{name} must declare properties");
        for (prop, def) in properties {
            assert!(
                def.get("default").is_some(),
                "{name}.{prop} must declare a default",
            );
        }
    }
}

#[test]
fn test_wasm_ecosystem_species_round_trip() {
    let mut world = make_world();
    let e = world.spawn_entity();
    world
        .set_component(e, "Species", &species_json("carnivore"))
        .unwrap();

    let raw = world
        .get_component(e, "Species")
        .expect("Species must round-trip");
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["diet"], "carnivore");
    assert!((parsed["graze_nutrition_rate"].as_f64().unwrap() - 0.2).abs() < 1e-9);
    assert_eq!(parsed["activity"], "nocturnal");

    assert!(world.get_entities_with_component("Species").contains(&e));
}

#[test]
fn test_wasm_ecosystem_wildlife_round_trip_with_defaults() {
    let mut world = make_world();
    let e = world.spawn_entity();
    let partial = serde_json::to_string(&serde_json::json!({
        "state": "wander",
        "satiety": 0.3,
    }))
    .unwrap();
    world.set_component(e, "Wildlife", &partial).unwrap();

    let raw = world
        .get_component(e, "Wildlife")
        .expect("Wildlife must round-trip");
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["state"], "wander");
    assert!((parsed["satiety"].as_f64().unwrap() - 0.3).abs() < 1e-9);
    assert_eq!(parsed["reproduction_cooldown"], 0);
    assert_eq!(parsed["flee_ticks"], 0);
    assert_eq!(parsed["rest_ticks"], 0);
}

#[test]
fn test_wasm_ecosystem_rejects_unknown_state() {
    let mut world = make_world();
    let e = world.spawn_entity();
    let bad = serde_json::to_string(&serde_json::json!({
        "state": "bogus",
        "satiety": 0.5,
        "reproduction_cooldown": 0,
        "flee_ticks": 0,
        "rest_ticks": 0,
    }))
    .unwrap();
    assert!(world.set_component(e, "Wildlife", &bad).is_err());
}

#[test]
fn test_wasm_ecosystem_entity_query_and_removal() {
    let mut world = make_world();
    let a = world.spawn_entity();
    let b = world.spawn_entity();
    let wildlife = serde_json::to_string(&serde_json::json!({
        "state": "graze",
        "satiety": 0.5,
        "reproduction_cooldown": 0,
        "flee_ticks": 0,
        "rest_ticks": 0,
    }))
    .unwrap();
    world.set_component(a, "Wildlife", &wildlife).unwrap();
    world.set_component(b, "Wildlife", &wildlife).unwrap();
    world
        .set_component(a, "Species", &species_json("herbivore"))
        .unwrap();

    let mut with_wildlife = world.get_entities_with_component("Wildlife");
    with_wildlife.sort_unstable();
    assert_eq!(with_wildlife, vec![a, b]);
    assert_eq!(world.get_entities_with_component("Species"), vec![a]);

    world.remove_component(b, "Wildlife").unwrap();
    assert_eq!(world.get_entities_with_component("Wildlife"), vec![a]);
    assert!(world.get_component(b, "Wildlife").is_none());
}
