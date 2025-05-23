use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas")
}

#[test]
fn test_entities_in_multiple_regions() {
    let schemas = load_schemas_from_dir(schema_dir()).unwrap();
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry);

    // eid1 in both "room_1" and "biome_A"
    let eid1 = world.spawn_entity();
    world
        .set_component(
            eid1,
            "Region",
            serde_json::json!({
                "id": ["room_1", "biome_A"],
                "kind": "room"
            }),
        )
        .unwrap();

    // eid2 only in "room_1"
    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "Region",
            serde_json::json!({
                "id": "room_1",
                "kind": "room"
            }),
        )
        .unwrap();

    // eid3 only in "biome_A"
    let eid3 = world.spawn_entity();
    world
        .set_component(
            eid3,
            "Region",
            serde_json::json!({
                "id": "biome_A",
                "kind": "biome"
            }),
        )
        .unwrap();

    // Query all entities in "room_1"
    let entities_room = world.entities_in_region("room_1");
    assert!(entities_room.contains(&eid1));
    assert!(entities_room.contains(&eid2));
    assert!(!entities_room.contains(&eid3));

    // Query all entities in "biome_A"
    let entities_biome = world.entities_in_region("biome_A");
    assert!(entities_biome.contains(&eid1));
    assert!(!entities_biome.contains(&eid2));
    assert!(entities_biome.contains(&eid3));
}
