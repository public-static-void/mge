#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::template::UnitTemplate;
use std::collections::HashMap;

fn setup_world_with_templates() -> engine_core::ecs::world::World {
    let mut world = world_helper::make_test_world();

    let mut components = HashMap::new();
    components.insert(
        "Health".to_string(),
        serde_json::json!({"current": 50, "max": 50}),
    );
    components.insert(
        "Renderable".to_string(),
        serde_json::json!({"glyph": "W", "color": [255, 0, 0]}),
    );

    world.template_registry.register_template(UnitTemplate {
        name: "warrior".to_string(),
        version: "1.0.0".to_string(),
        description: "Test warrior".to_string(),
        components,
    });

    world
}

#[test]
fn test_spawn_from_template_basic() {
    let mut world = setup_world_with_templates();
    let eid = world.spawn_from_template("warrior", None).unwrap();

    let health = world.get_component(eid, "Health").unwrap();
    assert_eq!(health["current"], 50);
    assert_eq!(health["max"], 50);

    let renderable = world.get_component(eid, "Renderable").unwrap();
    assert_eq!(renderable["glyph"], "W");
}

#[test]
fn test_spawn_from_template_with_overrides() {
    let mut world = setup_world_with_templates();
    let mut overrides = serde_json::Map::new();
    overrides.insert("Health".to_string(), serde_json::json!({"current": 100}));

    let eid = world
        .spawn_from_template("warrior", Some(overrides))
        .unwrap();

    let health = world.get_component(eid, "Health").unwrap();
    assert_eq!(health["current"], 100);
    assert_eq!(health["max"], 50);
}

#[test]
fn test_spawn_from_template_missing_name() {
    let mut world = setup_world_with_templates();
    let result = world.spawn_from_template("nonexistent", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_deep_merge_objects() {
    let mut base = serde_json::json!({"a": 1, "b": 2});
    let override_val = serde_json::json!({"b": 3, "c": 4});
    engine_core::ecs::world::World::deep_merge(&mut base, &override_val);
    assert_eq!(base["a"], 1);
    assert_eq!(base["b"], 3);
    assert_eq!(base["c"], 4);
}

#[test]
fn test_deep_merge_non_object_replaces() {
    let mut base = serde_json::json!([1, 2, 3]);
    let override_val = serde_json::json!([4, 5]);
    engine_core::ecs::world::World::deep_merge(&mut base, &override_val);
    assert_eq!(base, serde_json::json!([4, 5]));
}
