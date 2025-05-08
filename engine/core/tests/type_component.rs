// engine/core/tests/type_component.rs

use engine_core::scripting::World;
use serde_json::json;

#[test]
fn test_set_and_get_type_component() {
    let mut world = World::new();
    let id = world.spawn();
    let type_value = json!({ "kind": "player" });
    world.set_component(id, "Type", type_value.clone()).unwrap();

    let stored = world.get_component(id, "Type").unwrap();
    assert_eq!(stored, &type_value);
}
