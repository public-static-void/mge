def test_get_visible_cells_basic(make_world):
    world = make_world()
    eid = world.spawn_entity()
    # Set up a small open map
    for x in range(-10, 11):
        for y in range(-10, 11):
            world.add_cell(x, y, 0)
    for x in range(-10, 11):
        for y in range(-10, 11):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -10 <= nx <= 10 and -10 <= ny <= 10:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))

    world.set_sight(eid, 5)
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.tick()

    cells = world.get_visible_cells(eid)
    assert len(cells) > 0, "Should have visible cells"

def test_origin_visible(make_world):
    world = make_world()
    eid = world.spawn_entity()
    for x in range(-10, 11):
        for y in range(-10, 11):
            world.add_cell(x, y, 0)
    for x in range(-10, 11):
        for y in range(-10, 11):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -10 <= nx <= 10 and -10 <= ny <= 10:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))

    world.set_sight(eid, 5)
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.tick()

    assert world.is_visible(eid, 0, 0, 0) is True

def test_set_sight(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_sight(eid, 12)
    sight = world.get_sight(eid)
    assert sight is not None
    assert sight["range"] == 12

def test_get_sight_none(make_world):
    world = make_world()
    eid = world.spawn_entity()
    sight = world.get_sight(eid)
    assert sight is None

def test_set_sight_roundtrip(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_sight(eid, 8)
    sight = world.get_sight(eid)
    assert sight is not None
    assert sight["range"] == 8

def test_is_visible_no_sight(make_world):
    world = make_world()
    eid = world.spawn_entity()
    for x in range(-5, 6):
        for y in range(-5, 6):
            world.add_cell(x, y, 0)
    for x in range(-5, 6):
        for y in range(-5, 6):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -5 <= nx <= 5 and -5 <= ny <= 5:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))

    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.tick()

    assert world.is_visible(eid, 0, 0, 0) is False

def test_get_visible_cells_no_sight(make_world):
    world = make_world()
    eid = world.spawn_entity()
    for x in range(-5, 6):
        for y in range(-5, 6):
            world.add_cell(x, y, 0)
    for x in range(-5, 6):
        for y in range(-5, 6):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -5 <= nx <= 5 and -5 <= ny <= 5:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))

    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.tick()

    cells = world.get_visible_cells(eid)
    assert len(cells) == 0
