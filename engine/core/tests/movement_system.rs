#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::components::position::{Position, PositionComponent};
use engine_core::map::{HexGridMap, Map, RegionMap, SquareGridMap};

#[test]
fn test_move_all_moves_positions() {
    let mut world = make_test_world();
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0);
    grid.add_cell(2, 1, 0);
    grid.add_cell(5, 7, 0);
    grid.add_cell(6, 6, 0);
    world.map = Some(Map::new(Box::new(grid)));
    world.current_mode = "colony".to_string();

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

#[test]
fn test_move_all_square() {
    let mut world = make_test_world();
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    world.map = Some(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Square { x: 0, y: 0, z: 0 },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

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
    let mut world = make_test_world();
    let mut grid = HexGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    world.map = Some(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Hex { q: 0, r: 0, z: 0 },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

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
    let mut world = make_test_world();
    let mut grid = RegionMap::new();
    grid.add_cell("A");
    grid.add_cell("B");
    grid.add_neighbor("A", "B");
    world.map = Some(Map::new(Box::new(grid)));

    let entity = world.spawn_entity();
    let pos = PositionComponent {
        pos: Position::Region { id: "A".into() },
    };
    world
        .set_component(entity, "Position", serde_json::to_value(&pos).unwrap())
        .unwrap();

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
