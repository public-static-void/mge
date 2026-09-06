def _make_plane(world):
    for x in range(0, 3):
        for y in range(0, 3):
            world.add_cell(x, y, 0)
    for x in range(0, 3):
        for y in range(0, 3):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if 0 <= nx <= 2 and 0 <= ny <= 2:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))


def test_get_fluid_none(make_world):
    world = make_world()
    _make_plane(world)
    fluid = world.get_fluid({"Square": {"x": 0, "y": 0, "z": 0}})
    assert fluid is None


def test_get_fluid_roundtrip(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 0, "y": 0, "z": 0}},
        {"fluid": {"type": "water", "level": 5}},
    )
    fluid = world.get_fluid({"Square": {"x": 0, "y": 0, "z": 0}})
    assert fluid is not None
    assert fluid["type"] == "water"
    assert fluid["level"] == 5


def test_get_fluid_magma(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 1, "y": 1, "z": 0}},
        {"fluid": {"type": "magma", "level": 3}},
    )
    fluid = world.get_fluid({"Square": {"x": 1, "y": 1, "z": 0}})
    assert fluid is not None
    assert fluid["type"] == "magma"
    assert fluid["level"] == 3


def test_get_fluid_dual(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 2, "y": 2, "z": 0}},
        {"fluid": {"water": 2, "magma": 1}},
    )
    fluid = world.get_fluid({"Square": {"x": 2, "y": 2, "z": 0}})
    assert fluid is not None
    assert fluid["water"] == 2
    assert fluid["magma"] == 1


def test_fluid_spreads_after_tick(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 0, "y": 0, "z": 0}},
        {"fluid": {"type": "water", "level": 8}},
    )
    world.tick()
    neighbor = world.get_fluid({"Square": {"x": 1, "y": 0, "z": 0}})
    assert neighbor is not None
    assert neighbor["level"] > 0


def test_water_defaults_to_fresh_shallow_flowing(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 0, "y": 0, "z": 0}},
        {"fluid": {"type": "water", "level": 4}},
    )
    world.tick()
    fluid = world.get_fluid({"Square": {"x": 0, "y": 0, "z": 0}})
    assert fluid is not None
    assert fluid["water_type"] == "fresh"
    assert fluid["depth"] == "shallow"
    assert fluid["flow_state"] == "flowing"


def test_taxonomy_fields_round_trip(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 0, "y": 0, "z": 0}},
        {
            "fluid": {
                "type": "water",
                "level": 4,
                "water_type": "salt",
                "depth": "deep",
                "flow_state": "stale",
            }
        },
    )
    world.tick()
    fluid = world.get_fluid({"Square": {"x": 0, "y": 0, "z": 0}})
    assert fluid is not None
    assert fluid["water_type"] == "salt"
    assert fluid["depth"] == "deep"
    assert fluid["flow_state"] == "stale"


def test_magma_has_no_taxonomy(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 1, "y": 1, "z": 0}},
        {"fluid": {"type": "magma", "level": 3}},
    )
    world.tick()
    fluid = world.get_fluid({"Square": {"x": 1, "y": 1, "z": 0}})
    assert fluid is not None
    assert fluid["type"] == "magma"
    assert "water_type" not in fluid
    assert "depth" not in fluid
    assert "flow_state" not in fluid


def test_mixing_fresh_into_salt_yields_brackish(make_world):
    world = make_world()
    _make_plane(world)
    world.set_cell_metadata(
        {"Square": {"x": 0, "y": 0, "z": 0}},
        {"fluid": {"type": "water", "level": 8, "water_type": "fresh"}},
    )
    world.set_cell_metadata(
        {"Square": {"x": 1, "y": 0, "z": 0}},
        {"fluid": {"type": "water", "level": 1, "water_type": "salt"}},
    )
    world.tick()
    receiver = world.get_fluid({"Square": {"x": 1, "y": 0, "z": 0}})
    assert receiver is not None
    assert receiver["water_type"] == "brackish"
