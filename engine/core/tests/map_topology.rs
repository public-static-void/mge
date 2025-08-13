use engine_core::map::{CellKey, HexGridMap, Map, ProvinceMap, SquareGridMap};

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
fn test_province_map() {
    let mut province = ProvinceMap::new();
    province.add_cell("A");
    province.add_cell("B");
    province.add_cell("C");
    province.add_neighbor("A", "B");
    province.add_neighbor("A", "C");

    let map = Map::new(Box::new(province));
    let cell = CellKey::Province {
        id: "A".to_string(),
    };
    let neighbors = map.neighbors(&cell);
    assert!(neighbors.contains(&CellKey::Province {
        id: "B".to_string()
    }));
    assert!(neighbors.contains(&CellKey::Province {
        id: "C".to_string()
    }));
}
