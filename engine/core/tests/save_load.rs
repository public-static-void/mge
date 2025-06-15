#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::components::position::{Position, PositionComponent};

#[test]
fn test_save_and_load_world_roundtrip() {
    let world = make_test_world();
    let registry = world.registry.clone();

    let mut world = world;
    world.current_mode = "roguelike".to_string();

    let e1 = world.spawn_entity();
    world
        .set_component(
            e1,
            "Health",
            serde_json::json!({ "current": 42, "max": 100 }),
        )
        .unwrap();

    let e2 = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Square { x: 1, y: 2, z: 0 },
    };
    world
        .set_component(e2, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

    let loaded_world = save_and_load_roundtrip(&world, registry);

    assert_eq!(world.entities, loaded_world.entities);
    assert_eq!(
        world.get_component(e1, "Health"),
        loaded_world.get_component(e1, "Health")
    );
    assert_eq!(
        world.get_component(e2, "Position"),
        loaded_world.get_component(e2, "Position")
    );
}
