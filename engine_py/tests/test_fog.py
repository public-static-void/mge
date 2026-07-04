# Tests for the fog-of-war system.
# Uses the make_world fixture from conftest.py.


def test_is_explored_after_tick(make_world):
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

    assert world.is_explored(eid, 0, 0, 0) is True


def test_is_explored_false_for_unseen(make_world):
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

    assert world.is_explored(eid, 99, 99, 0) is False


def test_reset_fog(make_world):
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

    assert world.is_explored(eid, 0, 0, 0) is True
    world.reset_fog(eid)
    assert world.is_explored(eid, 0, 0, 0) is False


def test_get_explored_cells_after_tick(make_world):
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

    cells = world.get_explored_cells(eid)
    assert len(cells) > 0, "Should have explored cells after tick"


def test_get_visibility_state(make_world):
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

    # Origin should be VISIBLE (2)
    assert world.get_visibility_state(eid, 0, 0, 0) == 2
    # Far-away cell should be UNEXPLORED (0)
    assert world.get_visibility_state(eid, 99, 99, 0) == 0


def test_fog_accumulation(make_world):
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

    world.set_sight(eid, 3)
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.tick()

    # Cell at distance 3 should be explored
    assert world.is_explored(eid, 3, 0, 0) is True

    # Move away
    world.set_component(eid, "Position", {"pos": {"Square": {"x": 10, "y": 10, "z": 0}}})
    world.tick()

    # Old cell should remain explored
    assert world.is_explored(eid, 3, 0, 0) is True
