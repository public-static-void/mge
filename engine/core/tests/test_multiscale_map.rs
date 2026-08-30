use engine_core::map::{CellKey, Map, ProvinceMap, SquareGridMap};

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

/// Build a square-topology map with a couple of cells.
fn square_map() -> Map {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    Map::new(Box::new(grid))
}

/// Build a province-topology (overmap) map with a couple of provinces.
fn province_map() -> Map {
    let mut grid = ProvinceMap::new();
    grid.add_cell("prov_a");
    grid.add_cell("prov_b");
    Map::new(Box::new(grid))
}

// AC001 (R001): two named maps registered, both in get_map_names() and World.maps.
#[test]
fn test_register_two_maps_appear_in_registry() {
    let mut world = make_test_world();

    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    let names = world.get_map_names();
    assert!(names.contains(&"overmap".to_string()));
    assert!(names.contains(&"local".to_string()));
    assert_eq!(names.len(), 2);

    assert!(world.maps.contains_key("overmap"));
    assert!(world.maps.contains_key("local"));
    assert_eq!(world.maps["overmap"].topology_type(), "province");
    assert_eq!(world.maps["local"].topology_type(), "square");
}

// AC002 (R002): after set_active_map, world.map reflects the selected map's topology.
#[test]
fn test_set_active_map_reflects_topology() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    world.set_active_map("local").unwrap();
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");

    world.set_active_map("overmap").unwrap();
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "province");
}

// AC003 (R002): set_active_map on an unknown name returns Err and leaves active map unchanged.
#[test]
fn test_set_active_map_unknown_name_errors_and_unchanged() {
    let mut world = make_test_world();
    world.register_map("local", square_map()).unwrap();
    world.set_active_map("local").unwrap();

    let err = world.set_active_map("nonexistent");
    assert!(err.is_err());

    // Active map unchanged: still "local" with square topology.
    assert_eq!(world.get_active_map_name(), "local");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");
}

// AC004 (R003): get_active_map_name returns the last successfully selected map;
// get_map_names contains exactly the registered names.
#[test]
fn test_active_map_name_and_exact_names() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    // No map selected yet -> empty active name.
    assert_eq!(world.get_active_map_name(), "");

    world.set_active_map("local").unwrap();
    assert_eq!(world.get_active_map_name(), "local");

    world.set_active_map("overmap").unwrap();
    assert_eq!(world.get_active_map_name(), "overmap");

    let mut names = world.get_map_names();
    names.sort();
    assert_eq!(names, vec!["local".to_string(), "overmap".to_string()]);
}

// AC017 (NFR004): duplicate register_map returns Err (no panic).
#[test]
fn test_register_duplicate_name_errors() {
    let mut world = make_test_world();
    world.register_map("local", square_map()).unwrap();

    let err = world.register_map("local", square_map());
    assert!(err.is_err());

    // Registry still holds the original map.
    assert_eq!(world.maps.len(), 1);
    assert_eq!(world.maps["local"].topology_type(), "square");
}

// --- M2: scale-generic link-based transitions + topology-generic coordinate mapping ---

// A square-topology map can serve as an overmap (Dwarf Fortress/Cataclysm style):
// topology and map type are decoupled.
#[test]
fn test_square_topology_named_overmap() {
    let mut world = make_test_world();
    world.register_map("overmap", square_map()).unwrap();
    assert_eq!(world.maps["overmap"].topology_type(), "square");
}

// A province-topology map can serve as a strategic map (Hearts of Iron style).
#[test]
fn test_province_topology_named_strategic() {
    let mut world = make_test_world();
    world.register_map("strategic", province_map()).unwrap();
    assert_eq!(world.maps["strategic"].topology_type(), "province");
}

// enter_map switches the active map to the target and positions the camera at the entry cell.
#[test]
fn test_enter_map_switches_active_and_positions_camera() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();
    world.set_active_map("overmap").unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let entry_cell = CellKey::Square { x: 1, y: 0, z: 0 };
    world
        .link_maps("overmap", source_cell, "field", entry_cell.clone())
        .unwrap();

    world.enter_map("field", entry_cell.clone()).unwrap();

    assert_eq!(world.get_active_map_name(), "field");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");

    // Camera position equals the entry cell.
    let camera_id = world
        .get_entities_with_component("Camera")
        .first()
        .cloned()
        .unwrap();
    let pos = world.get_component(camera_id, "Position").unwrap();
    assert_eq!(
        pos,
        &serde_json::json!({ "pos": { "Square": { "x": 1, "y": 0, "z": 0 } } })
    );
}

// exit_map returns to the previously active map.
#[test]
fn test_exit_map_returns_to_previous_map() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();
    world.set_active_map("overmap").unwrap();

    world
        .enter_map("field", CellKey::Square { x: 0, y: 0, z: 0 })
        .unwrap();
    assert_eq!(world.get_active_map_name(), "field");

    world.exit_map().unwrap();
    assert_eq!(world.get_active_map_name(), "overmap");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "province");
}

// enter_map on an unknown map name errors; exit_map with an empty stack errors.
#[test]
fn test_enter_unknown_map_and_exit_empty_stack_error() {
    let mut world = make_test_world();
    world.register_map("field", square_map()).unwrap();

    let err = world.enter_map("nonexistent", CellKey::Square { x: 0, y: 0, z: 0 });
    assert!(err.is_err());

    let err = world.exit_map();
    assert!(err.is_err());
}

// map_cell/unmap_cell round-trip a linked source cell to its target cell and back.
#[test]
fn test_map_cell_unmap_cell_round_trip() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let target_cell = CellKey::Square { x: 2, y: 1, z: 0 };
    world
        .link_maps("overmap", source_cell.clone(), "field", target_cell.clone())
        .unwrap();

    assert_eq!(
        world.map_cell("overmap", &source_cell),
        Some(target_cell.clone())
    );
    assert_eq!(
        world.unmap_cell("field", &target_cell),
        Some(source_cell.clone())
    );
}

// map_cell/unmap_cell return None for unlinked source/target cells.
#[test]
fn test_map_cell_unmap_cell_unlinked_returns_none() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let target_cell = CellKey::Square { x: 2, y: 1, z: 0 };
    world
        .link_maps("overmap", source_cell, "field", target_cell)
        .unwrap();

    // Unlinked source cell on a linked map.
    assert_eq!(
        world.map_cell(
            "overmap",
            &CellKey::Province {
                id: "prov_b".to_string()
            }
        ),
        None
    );
    // Unlinked map entirely.
    assert_eq!(
        world.map_cell("field", &CellKey::Square { x: 0, y: 0, z: 0 }),
        None
    );
    // Unlinked target cell.
    assert_eq!(
        world.unmap_cell("field", &CellKey::Square { x: 9, y: 9, z: 0 }),
        None
    );
    // Unlinked target map.
    assert_eq!(
        world.unmap_cell(
            "overmap",
            &CellKey::Province {
                id: "prov_a".to_string()
            }
        ),
        None
    );
}

// Coordinate mapping is topology-generic: a province source cell round-trips to a
// square target cell across two different topologies.
#[test]
fn test_cross_topology_round_trip_province_to_square() {
    let mut world = make_test_world();
    world.register_map("strategic", province_map()).unwrap();
    world.register_map("tactical", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_b".to_string(),
    };
    let target_cell = CellKey::Square { x: 3, y: 4, z: 0 };
    world
        .link_maps(
            "strategic",
            source_cell.clone(),
            "tactical",
            target_cell.clone(),
        )
        .unwrap();

    assert_eq!(
        world.map_cell("strategic", &source_cell),
        Some(target_cell.clone())
    );
    assert_eq!(
        world.unmap_cell("tactical", &target_cell),
        Some(source_cell.clone())
    );
}

// link_maps with an unknown map name errors (no panic).
#[test]
fn test_link_maps_unknown_map_errors() {
    let mut world = make_test_world();
    world.register_map("field", square_map()).unwrap();

    let err = world.link_maps(
        "nonexistent",
        CellKey::Square { x: 0, y: 0, z: 0 },
        "field",
        CellKey::Square { x: 1, y: 0, z: 0 },
    );
    assert!(err.is_err());

    let err = world.link_maps(
        "field",
        CellKey::Square { x: 0, y: 0, z: 0 },
        "nonexistent",
        CellKey::Square { x: 1, y: 0, z: 0 },
    );
    assert!(err.is_err());
}
