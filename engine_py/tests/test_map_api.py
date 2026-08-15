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


def test_entities_in_zlevel_returns_matching_entities(make_world):
    world = make_world()
    world.add_cell(2, 0, 1)

    lower = world.spawn_entity()
    world.set_component(lower, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    upper = world.spawn_entity()
    world.set_component(upper, "Position", {"pos": {"Square": {"x": 2, "y": 0, "z": 1}}})

    on_zero = world.entities_in_zlevel(0)
    assert lower in on_zero, "Lower entity should be on z-level 0"
    assert upper not in on_zero, "Upper entity should not be on z-level 0"

    on_one = world.entities_in_zlevel(1)
    assert upper in on_one, "Upper entity should be on z-level 1"
    assert lower not in on_one, "Lower entity should not be on z-level 1"


def test_move_entity_3d_changes_z(make_world):
    world = make_world()
    world.add_cell(0, 0, 0)
    world.add_cell(0, 0, 2)

    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})

    world.move_entity_3d(eid, 0, 0, 2)

    pos = world.get_component(eid, "Position")
    assert pos["pos"]["Square"]["x"] == 0, "x should be unchanged by vertical movement"
    assert pos["pos"]["Square"]["y"] == 0, "y should be unchanged by vertical movement"
    assert pos["pos"]["Square"]["z"] == 2, "z should increase by dz"

    assert eid in world.entities_in_zlevel(2), "Entity should be found on new z-level"
    assert eid not in world.entities_in_zlevel(0), "Entity should leave old z-level"


def test_move_entity_3d_shifts_xy_and_z(make_world):
    world = make_world()

    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 1, "y": 1, "z": 0}}})

    world.move_entity_3d(eid, 2, 3, 1)

    pos = world.get_component(eid, "Position")
    assert pos["pos"]["Square"]["x"] == 3, "x should shift by dx"
    assert pos["pos"]["Square"]["y"] == 4, "y should shift by dy"
    assert pos["pos"]["Square"]["z"] == 1, "z should shift by dz"
