use engine_core::map::{CellKey, HexGridMap, Map, RegionMap, SquareGridMap};

#[test]
fn test_square_grid_map() {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_cell(0, 1, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    grid.add_neighbor((0, 0, 0), (0, 1, 0));

    let map = Map::new(Box::new(grid));
    let cell = CellKey::Square { x: 0, y: 0, z: 0 };
    let neighbors = map.neighbors(&cell);
    assert!(neighbors.contains(&CellKey::Square { x: 1, y: 0, z: 0 }));
    assert!(neighbors.contains(&CellKey::Square { x: 0, y: 1, z: 0 }));
}

#[test]
fn test_hex_grid_map() {
    let mut grid = HexGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));

    let map = Map::new(Box::new(grid));
    let cell = CellKey::Hex { q: 0, r: 0, z: 0 };
    let neighbors = map.neighbors(&cell);
    assert_eq!(neighbors, vec![CellKey::Hex { q: 1, r: 0, z: 0 }]);
}

#[test]
fn test_region_map() {
    let mut region = RegionMap::new();
    region.add_cell("A");
    region.add_cell("B");
    region.add_cell("C");
    region.add_neighbor("A", "B");
    region.add_neighbor("A", "C");

    let map = Map::new(Box::new(region));
    let cell = CellKey::Region {
        id: "A".to_string(),
    };
    let neighbors = map.neighbors(&cell);
    assert!(neighbors.contains(&CellKey::Region {
        id: "B".to_string()
    }));
    assert!(neighbors.contains(&CellKey::Region {
        id: "C".to_string()
    }));
}
