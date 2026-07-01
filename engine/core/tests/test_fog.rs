//! Integration tests for the Fog of War system.
//!
//! Tests fog state initialization, visible→explored merge, fog accumulation,
//! save/load roundtrip, backward-compatible deserialization, and system behavior.

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::fog::FogUpdateSystem;
use engine_core::systems::fov::FovUpdateSystem;
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Build an open plane map with 8-directional adjacency.
fn open_plane(size: i32) -> Map {
    let mut grid = SquareGridMap::new();
    for x in -size..=size {
        for y in -size..=size {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -size..=size {
        for y in -size..=size {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= -size && nx <= size && ny >= -size && ny <= size {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    Map::new(Box::new(grid))
}

fn setup_world_with_schema() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Sight".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/sight.json")).unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
        reg.register_external_schema(ComponentSchema {
            name: "Position".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/position.json"))
                .unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
    }
    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();
    world
}

#[test]
fn integration_fog_initially_empty() {
    let world = setup_world_with_schema();
    assert!(
        world.explored_cells.is_empty(),
        "explored_cells should be empty in a new world"
    );
}

#[test]
fn integration_fog_get_none_for_nonexistent() {
    let world = setup_world_with_schema();
    assert_eq!(world.get_explored_cells(1), None);
}

#[test]
fn integration_fog_set_get_roundtrip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    cells.insert(CellKey::Square { x: 1, y: 0, z: 0 });
    world.set_explored_cells(42, cells.clone());
    let retrieved = world.get_explored_cells(42);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), &cells);
}

#[test]
fn integration_reset_fog_clears_entity() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 5, y: 5, z: 0 });
    world.set_explored_cells(7, cells);
    assert!(world.get_explored_cells(7).is_some());
    world.reset_fog(7);
    assert_eq!(world.get_explored_cells(7), None);
}

#[test]
fn integration_reset_all_fog_clears_all() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    world.set_explored_cells(1, cells.clone());
    world.set_explored_cells(2, cells.clone());
    assert_eq!(world.explored_cells.len(), 2);
    world.reset_all_fog();
    assert!(world.explored_cells.is_empty());
}

#[test]
fn integration_fog_update_system_merges_visible_to_explored() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(10));

    let e = world.spawn_entity();
    world
        .set_component(e, "Sight", json!({"range": 5}))
        .unwrap();
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    // Run FOV first, then fog
    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored = world.get_explored_cells(e);
    assert!(
        explored.is_some(),
        "Explored cells should be computed after fog update"
    );
    let cells = explored.unwrap();
    assert!(
        cells.contains(&CellKey::Square { x: 0, y: 0, z: 0 }),
        "Origin must be in explored cells"
    );
    assert!(cells.len() > 1, "Multiple cells should be explored");
}

#[test]
fn integration_fog_accumulation_across_ticks() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(10));

    let e = world.spawn_entity();
    world
        .set_component(e, "Sight", json!({"range": 3}))
        .unwrap();
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();

    // Tick 1: FOV + fog at (0,0)
    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored_tick1 = world.get_explored_cells(e).unwrap().clone();
    assert!(explored_tick1.contains(&CellKey::Square { x: 3, y: 0, z: 0 }));

    // Move to (10,0) — old cells go out of FOV
    world
        .set_component(
            e,
            "Position",
            json!({"pos": {"Square": {"x": 10, "y": 0, "z": 0}}}),
        )
        .unwrap();

    // Tick 2: FOV + fog at (10,0)
    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    let explored_tick2 = world.get_explored_cells(e).unwrap();
    // Old cells should still be explored
    assert!(
        explored_tick2.contains(&CellKey::Square { x: 3, y: 0, z: 0 }),
        "Cell (3,0) should remain explored after moving away"
    );
    // New cells should also be explored
    assert!(
        explored_tick2.contains(&CellKey::Square { x: 10, y: 0, z: 0 }),
        "Cell (10,0) should be explored after moving"
    );
}

#[test]
fn integration_fog_serialization_roundtrip() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 10, y: 20, z: 0 });
    world.set_explored_cells(5, cells);

    let json = serde_json::to_string(&world).unwrap();
    let deserialized: World = serde_json::from_str(&json).unwrap();

    let retrieved = deserialized.get_explored_cells(5);
    assert!(
        retrieved.is_some(),
        "explored_cells should survive serialization"
    );
    assert!(
        retrieved
            .unwrap()
            .contains(&CellKey::Square { x: 10, y: 20, z: 0 })
    );
}

#[test]
fn integration_fog_serialization_backward_compat() {
    // Simulate an old save file that has no explored_cells field
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 1, y: 2, z: 0 });
    world.set_explored_cells(3, cells);

    // Serialize, then remove explored_cells to simulate old format
    let mut json_value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    if let Some(obj) = json_value.as_object_mut() {
        obj.remove("explored_cells");
    }
    let old_json = serde_json::to_string(&json_value).unwrap();

    // Deserialize — should not error, explored_cells should be empty
    let deserialized: World = serde_json::from_str(&old_json).unwrap();
    assert!(
        deserialized.explored_cells.is_empty(),
        "Old save without explored_cells should deserialize to empty fog state"
    );
}

#[test]
fn integration_no_sight_gets_no_fog() {
    let mut world = setup_world_with_schema();
    world.map = Some(open_plane(5));

    let e = world.spawn_entity();
    // No Sight component
    let mut fov = FovUpdateSystem;
    fov.run(&mut world);
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);

    assert!(world.get_explored_cells(e).is_none());
}

#[test]
fn integration_fog_system_name() {
    let system = FogUpdateSystem;
    assert_eq!(system.name(), "FogUpdateSystem");
}

#[test]
fn integration_fog_system_dependencies() {
    let system = FogUpdateSystem;
    assert_eq!(system.dependencies(), &["FovUpdateSystem"]);
}

#[test]
fn integration_fog_system_no_map() {
    let mut world = setup_world_with_schema();
    // No map set — should not panic
    let mut fog = FogUpdateSystem;
    fog.run(&mut world);
}

#[test]
fn integration_visibility_state_all_states() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 1, y: 0, z: 0 }); // explored but not visible
    world.set_explored_cells(1, cells);

    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 2, y: 0, z: 0 }); // currently visible
    world.set_visible_cells(1, visible);

    let cell_unexplored = CellKey::Square { x: 0, y: 0, z: 0 };
    let cell_explored = CellKey::Square { x: 1, y: 0, z: 0 };
    let cell_visible = CellKey::Square { x: 2, y: 0, z: 0 };

    assert_eq!(
        world.get_visibility_state(1, &cell_unexplored),
        0,
        "Unexplored cell"
    );
    assert_eq!(
        world.get_visibility_state(1, &cell_explored),
        1,
        "Explored cell"
    );
    assert_eq!(
        world.get_visibility_state(1, &cell_visible),
        2,
        "Visible cell"
    );
}

#[test]
fn integration_fog_get_explored_cells_method() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 5, y: 5, z: 0 });
    world.set_explored_cells(10, cells.clone());
    assert_eq!(world.get_explored_cells(10), Some(&cells));
}

#[test]
fn integration_fog_set_explored_cells_method() {
    let mut world = setup_world_with_schema();
    let mut cells = HashSet::new();
    cells.insert(CellKey::Square { x: 3, y: 4, z: 0 });
    world.set_explored_cells(20, cells.clone());
    assert_eq!(world.get_explored_cells(20), Some(&cells));
}

#[test]
fn integration_fog_multiple_entities_independent() {
    let mut world = setup_world_with_schema();
    let mut cells_a = HashSet::new();
    cells_a.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    let mut cells_b = HashSet::new();
    cells_b.insert(CellKey::Square { x: 10, y: 10, z: 0 });

    world.set_explored_cells(100, cells_a);
    world.set_explored_cells(200, cells_b);

    assert_eq!(world.get_explored_cells(100).unwrap().len(), 1);
    assert_eq!(world.get_explored_cells(200).unwrap().len(), 1);
    assert!(
        !world
            .get_explored_cells(100)
            .unwrap()
            .contains(&CellKey::Square { x: 10, y: 10, z: 0 })
    );
}
