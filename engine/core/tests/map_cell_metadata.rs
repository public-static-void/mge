use engine_core::map::MapTopology;
use engine_core::map::{CellKey, SquareGridMap};
use serde_json::json;

#[test]
fn test_set_and_get_cell_metadata() {
    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0);
    let key = CellKey::Square { x: 1, y: 2, z: 0 };
    grid.set_cell_metadata(&key, json!({"biome": "Forest", "terrain": "Grass"}));
    let meta = grid.get_cell_metadata(&key).unwrap();
    assert_eq!(meta["biome"], "Forest");
    assert_eq!(meta["terrain"], "Grass");
}
