#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

#[test]
fn test_entities_in_multiple_regions() {
    let mut world = world_helper::make_test_world();

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

    let entities_room = world.entities_in_region("room_1");
    assert!(entities_room.contains(&eid1), "eid1 should be in room_1");
    assert!(entities_room.contains(&eid2), "eid2 should be in room_1");
    assert!(
        !entities_room.contains(&eid3),
        "eid3 should not be in room_1"
    );

    let entities_biome = world.entities_in_region("biome_A");
    assert!(entities_biome.contains(&eid1), "eid1 should be in biome_A");
    assert!(
        !entities_biome.contains(&eid2),
        "eid2 should not be in biome_A"
    );
    assert!(entities_biome.contains(&eid3), "eid3 should be in biome_A");
}

#[test]
fn test_get_entities_with_components() {
    let mut world = world_helper::make_test_world();

    let e1 = world.spawn_entity();
    let e2 = world.spawn_entity();
    let e3 = world.spawn_entity();

    world
        .set_component(e1, "Health", json!({"current": 10, "max": 10}))
        .unwrap();
    world
        .set_component(
            e1,
            "Position",
            json!({"pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();

    world
        .set_component(e2, "Health", json!({"current": 5, "max": 10}))
        .unwrap();

    world
        .set_component(
            e3,
            "Position",
            json!({"pos": { "Square": { "x": 3, "y": 4, "z": 0 } } }),
        )
        .unwrap();

    let both = world.get_entities_with_components(&["Health", "Position"]);
    assert_eq!(
        both,
        vec![e1],
        "Only e1 should have both Health and Position"
    );
}

#[test]
fn test_entities_by_region_kind() {
    let mut world = world_helper::make_test_world();

    let eid1 = world.spawn_entity();
    world
        .set_component(
            eid1,
            "Region",
            json!({
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
            json!({
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
            json!({
                "id": "room_2",
                "kind": "room"
            }),
        )
        .unwrap();

    let room_entities = world.entities_in_region_kind("room");
    assert!(room_entities.contains(&eid1), "eid1 should be in room kind");
    assert!(room_entities.contains(&eid3), "eid3 should be in room kind");
    assert!(
        !room_entities.contains(&eid2),
        "eid2 should not be in room kind"
    );

    let stockpile_entities = world.entities_in_region_kind("stockpile");
    assert!(
        stockpile_entities.contains(&eid2),
        "eid2 should be in stockpile kind"
    );
    assert!(
        !stockpile_entities.contains(&eid1),
        "eid1 should not be in stockpile kind"
    );
}
