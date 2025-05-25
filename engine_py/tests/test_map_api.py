def test_map_api(make_world):
    world = make_world()
    world.add_cell(0, 0, 0)
    world.add_cell(1, 0, 0)
    world.add_cell(0, 1, 0)

    # Add neighbors explicitly
    world.add_neighbor((0, 0, 0), (1, 0, 0))
    world.add_neighbor((0, 0, 0), (0, 1, 0))

    topo = world.get_map_topology_type()
    assert topo == "square"

    cells = world.get_all_cells()
    assert len(cells) >= 3

    cell = {"Square": {"x": 0, "y": 0, "z": 0}}
    neighbors = world.get_neighbors(cell)
    assert len(neighbors) > 0
