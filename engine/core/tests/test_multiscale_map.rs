use engine_core::map::{Map, ProvinceMap, SquareGridMap};

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
