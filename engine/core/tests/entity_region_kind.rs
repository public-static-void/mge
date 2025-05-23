use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas")
}

#[test]
fn test_entities_by_region_kind() {
    let schemas = load_schemas_from_dir(schema_dir()).unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    let eid1 = world.spawn_entity();
    world
        .set_component(
            eid1,
            "Region",
            serde_json::json!({
                "id": "room_1",
                "kind": "room"
            }),
        )
        .unwrap();

    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "Region",
            serde_json::json!({
                "id": "stockpile_1",
                "kind": "stockpile"
            }),
        )
        .unwrap();

    let eid3 = world.spawn_entity();
    world
        .set_component(
            eid3,
            "Region",
            serde_json::json!({
                "id": "room_2",
                "kind": "room"
            }),
        )
        .unwrap();

    let room_entities = world.entities_in_region_kind("room");
    assert!(room_entities.contains(&eid1));
    assert!(room_entities.contains(&eid3));
    assert!(!room_entities.contains(&eid2));

    let stockpile_entities = world.entities_in_region_kind("stockpile");
    assert!(stockpile_entities.contains(&eid2));
    assert!(!stockpile_entities.contains(&eid1));
}
