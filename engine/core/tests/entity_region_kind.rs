#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

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
