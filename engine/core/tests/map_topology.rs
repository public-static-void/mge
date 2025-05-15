use engine_core::map::Map;

#[test]
fn test_add_cells_and_neighbors_square() {
    let mut map = Map::new("square");
    map.add_cell("A");
    map.add_cell("B");
    map.add_cell("C");

    map.add_neighbor("A", "B");
    map.add_neighbor("A", "C");

    let neighbors = map.neighbors("A").unwrap();
    assert!(neighbors.contains("B"));
    assert!(neighbors.contains("C"));
    assert_eq!(neighbors.len(), 2);
}

#[test]
fn test_hex_topology_cells() {
    let mut map = Map::new("hex");
    map.add_cell("H1");
    map.add_cell("H2");

    map.add_neighbor("H1", "H2");

    let neighbors = map.neighbors("H1").unwrap();
    assert!(neighbors.contains("H2"));
    assert_eq!(neighbors.len(), 1);
}

#[test]
fn test_arbitrary_graph_topology() {
    let mut map = Map::new("graph");
    map.add_cell("X");
    map.add_cell("Y");
    map.add_cell("Z");

    map.add_neighbor("X", "Y");
    map.add_neighbor("Y", "Z");

    let neighbors_x = map.neighbors("X").unwrap();
    let neighbors_y = map.neighbors("Y").unwrap();

    assert!(neighbors_x.contains("Y"));
    assert!(neighbors_y.contains("Z"));
    assert_eq!(neighbors_x.len(), 1);
    assert_eq!(neighbors_y.len(), 1);
}
