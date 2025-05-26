use engine_core::map::{CellKey, Map, MapTopology, SquareGridMap};
use serde_json::json;

#[test]
fn test_pathfinding_simple() {
    let mut grid = SquareGridMap::new();
    for x in 0..3 {
        for y in 0..3 {
            grid.add_cell(x, y, 0);
        }
    }
    // Connect all neighbors (4-way)
    for x in 0..3 {
        for y in 0..3 {
            let from = (x, y, 0);
            for (dx, dy) in &[(1, 0), (0, 1), (-1, 0), (0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                if (0..3).contains(&nx) && (0..3).contains(&ny) {
                    grid.add_neighbor(from, (nx, ny, 0));
                }
            }
        }
    }
    // Block (1,1,0)
    let block = CellKey::Square { x: 1, y: 1, z: 0 };
    grid.set_cell_metadata(&block, json!({"walkable": false}));

    let map = Map::new(Box::new(grid));
    let start = CellKey::Square { x: 0, y: 0, z: 0 };
    let goal = CellKey::Square { x: 2, y: 2, z: 0 };
    let result = map.find_path(&start, &goal).expect("Path should exist");
    // Path should not go through (1,1,0)
    assert!(!result.path.contains(&block));
    // Path length should be 5 (around the block)
    assert_eq!(result.path.len(), 5);
}
