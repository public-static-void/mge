#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

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
