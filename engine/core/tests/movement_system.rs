use engine_core::ecs::components::position::{Position, PositionComponent};
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir;
use engine_core::ecs::world::World;
use engine_core::map::{HexGridMap, Map, RegionMap, SquareGridMap};
use std::sync::{Arc, Mutex};

#[test]
fn test_move_all_moves_positions() {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Setup map with both the original and target squares
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0); // initial pos1
    grid.add_cell(2, 1, 0); // target pos1
    grid.add_cell(5, 7, 0); // initial pos2
    grid.add_cell(6, 6, 0); // target pos2
    world.map = Some(Map::new(Box::new(grid)));

    let id1 = world.spawn_entity();
    let id2 = world.spawn_entity();

    let pos1 = PositionComponent {
        pos: Position::Square { x: 1, y: 2, z: 0 },
    };
    let pos2 = PositionComponent {
        pos: Position::Square { x: 5, y: 7, z: 0 },
    };
    world
        .set_component(id1, "Position", serde_json::to_value(&pos1).unwrap())
        .unwrap();
    world
        .set_component(id2, "Position", serde_json::to_value(&pos2).unwrap())
        .unwrap();

    // Batch move: increment x by 1, y by -1 for all Square positions
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value.clone()) {
                if let Position::Square { x, y, z: _ } = &mut pos_comp.pos {
                    *x += 1;
                    *y += -1;
                }
                *value = serde_json::to_value(&pos_comp).unwrap();
            }
        }
    }

    let pos1_val = world.get_component(id1, "Position").unwrap().clone();
    let pos2_val = world.get_component(id2, "Position").unwrap().clone();

    let pos1: PositionComponent = serde_json::from_value(pos1_val).unwrap();
    let pos2: PositionComponent = serde_json::from_value(pos2_val).unwrap();

    assert_eq!(pos1.pos, Position::Square { x: 2, y: 1, z: 0 });
    assert_eq!(pos2.pos, Position::Square { x: 6, y: 6, z: 0 });
}

fn setup_world_with_map(map: Map) -> World {
    let schema_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/../assets/schemas";
    let schemas = load_schemas_from_dir(&schema_dir).expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = std::sync::Arc::new(std::sync::Mutex::new(registry));
    let mut world = World::new(registry);
    world.map = Some(map);
    world
}

#[test]
fn test_move_all_square() {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    let mut world = setup_world_with_map(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Square { x: 0, y: 0, z: 0 },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

    // Move all entities with Position (Square) by dx=1, dy=0, dz=0
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value.clone()) {
                if let Position::Square { x, y: _, z: _ } = &mut pos_comp.pos {
                    *x += 1;
                }
                *value = serde_json::to_value(&pos_comp).unwrap();
            }
        }
    }

    let new_pos: PositionComponent =
        serde_json::from_value(world.get_component(entity, "Position").unwrap().clone())
            .expect("valid Position");
    assert_eq!(new_pos.pos, Position::Square { x: 1, y: 0, z: 0 });
}

#[test]
fn test_move_all_hex() {
    let mut grid = HexGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    let mut world = setup_world_with_map(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Hex { q: 0, r: 0, z: 0 },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

    // Move all entities with Position (Hex) by dq=1, dr=0, dz=0
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value.clone()) {
                if let Position::Hex { q, r: _, z: _ } = &mut pos_comp.pos {
                    *q += 1;
                }
                *value = serde_json::to_value(&pos_comp).unwrap();
            }
        }
    }

    let new_pos: PositionComponent =
        serde_json::from_value(world.get_component(entity, "Position").unwrap().clone())
            .expect("valid Position");
    assert_eq!(new_pos.pos, Position::Hex { q: 1, r: 0, z: 0 });
}

#[test]
fn test_move_all_region() {
    let mut grid = RegionMap::new();
    grid.add_cell("A");
    grid.add_cell("B");
    grid.add_neighbor("A", "B");
    let mut world = setup_world_with_map(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Region { id: "A".into() },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

    // Move all entities with Position (Region) to id "B"
    if let Some(positions) = world.components.get_mut("Position") {
        for (_eid, value) in positions.iter_mut() {
            if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value.clone()) {
                if let Position::Region { id } = &mut pos_comp.pos {
                    *id = "B".into();
                }
                *value = serde_json::to_value(&pos_comp).unwrap();
            }
        }
    }

    let new_pos: PositionComponent =
        serde_json::from_value(world.get_component(entity, "Position").unwrap().clone())
            .expect("valid Position");
    assert_eq!(new_pos.pos, Position::Region { id: "B".into() });
}
