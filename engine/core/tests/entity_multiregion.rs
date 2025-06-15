#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

#[test]
fn test_entities_in_multiple_regions() {
    let mut world = world_helper::make_test_world();

    // eid1 in both "room_1" and "biome_A"
    let eid1 = world.spawn_entity();
    world
        .set_component(
            eid1,
            "Region",
            json!({
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
            json!({
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
            json!({
                "id": "biome_A",
                "kind": "biome"
            }),
        )
        .unwrap();

    // Query all entities in "room_1"
    let entities_room = world.entities_in_region("room_1");
    assert!(entities_room.contains(&eid1), "eid1 should be in room_1");
    assert!(entities_room.contains(&eid2), "eid2 should be in room_1");
    assert!(
        !entities_room.contains(&eid3),
        "eid3 should not be in room_1"
    );

    // Query all entities in "biome_A"
    let entities_biome = world.entities_in_region("biome_A");
    assert!(entities_biome.contains(&eid1), "eid1 should be in biome_A");
    assert!(
        !entities_biome.contains(&eid2),
        "eid2 should not be in biome_A"
    );
    assert!(entities_biome.contains(&eid3), "eid3 should be in biome_A");
}
