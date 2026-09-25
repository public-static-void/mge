"""Parity tests for the ecosystem and wildlife simulation via the Python API."""


def _make_open_plane(world, size=10):
    for x in range(-size, size + 1):
        for y in range(-size, size + 1):
            world.add_cell(x, y, 0)
    for x in range(-size, size + 1):
        for y in range(-size, size + 1):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -size <= nx <= size and -size <= ny <= size:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))


def _set_species(world, eid, overrides=None):
    base = {
        "diet": "herbivore",
        "graze_nutrition_rate": 0.2,
        "metabolism_rate": 0.05,
        "reproduction_threshold": 0.8,
        "reproduction_cooldown_ticks": 10,
        "litter_size": 1,
        "detection_range": 6,
        "noise_flee_threshold": 0.5,
        "activity": "nocturnal",
    }
    if overrides:
        base.update(overrides)
    world.set_component(eid, "Species", base)


def _spawn_wildlife(world, x, y, state, satiety, species_overrides=None):
    eid = world.spawn_entity()
    world.set_component(
        eid,
        "Wildlife",
        {
            "state": state,
            "satiety": satiety,
            "reproduction_cooldown": 0,
            "flee_ticks": 0,
            "rest_ticks": 0,
        },
    )
    world.set_component(eid, "Position", {"pos": {"Square": {"x": x, "y": y, "z": 0}}})
    _set_species(world, eid, species_overrides)
    return eid


def test_species_wildlife_round_trip_with_defaults(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(
        eid,
        "Species",
        {
            "diet": "carnivore",
            "graze_nutrition_rate": 0.2,
            "metabolism_rate": 0.05,
            "reproduction_threshold": 0.8,
            "reproduction_cooldown_ticks": 10,
            "litter_size": 1,
            "detection_range": 6,
            "noise_flee_threshold": 0.5,
            "activity": "nocturnal",
        },
    )
    species = world.get_component(eid, "Species")
    assert species is not None
    assert species["diet"] == "carnivore"

    world.set_component(eid, "Wildlife", {"state": "wander", "satiety": 0.3})
    wildlife = world.get_component(eid, "Wildlife")
    assert wildlife is not None
    assert wildlife["state"] == "wander"
    assert abs(wildlife["satiety"] - 0.3) < 1e-9
    assert wildlife["reproduction_cooldown"] == 0
    assert wildlife["flee_ticks"] == 0
    assert wildlife["rest_ticks"] == 0


def test_nocturnal_graze_gain_exact_and_stationary(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = _spawn_wildlife(world, 0, 0, "graze", 0.5)

    world.tick()

    wildlife = world.get_component(eid, "Wildlife")
    assert abs(wildlife["satiety"] - 0.7) < 1e-9
    pos = world.get_component(eid, "Position")
    assert pos["pos"]["Square"]["x"] == 0
    assert pos["pos"]["Square"]["y"] == 0


def test_graze_gain_clamps_at_one(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = _spawn_wildlife(world, 0, 0, "graze", 0.95)
    world.set_component(
        eid,
        "Wildlife",
        {
            "state": "graze",
            "satiety": 0.95,
            "reproduction_cooldown": 5,
            "flee_ticks": 0,
            "rest_ticks": 0,
        },
    )

    world.tick()

    wildlife = world.get_component(eid, "Wildlife")
    assert wildlife["satiety"] == 1.0


def test_diurnal_rests_during_night(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = _spawn_wildlife(world, 0, 0, "graze", 0.5, {"activity": "diurnal"})

    world.tick()

    wildlife = world.get_component(eid, "Wildlife")
    assert wildlife["state"] == "rest"
    pos = world.get_component(eid, "Position")
    assert pos["pos"]["Square"]["x"] == 0
    assert pos["pos"]["Square"]["y"] == 0


def test_flee_on_noise(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = _spawn_wildlife(world, 0, 0, "graze", 0.5)
    assert world.emit_noise(eid, 1.0, 5) is True

    world.tick()

    wildlife = world.get_component(eid, "Wildlife")
    assert wildlife["state"] == "flee"
    pos = world.get_component(eid, "Position")
    assert (pos["pos"]["Square"]["x"], pos["pos"]["Square"]["y"]) != (0, 0)


def test_reproduce_spawns_child_and_updates_parent(make_world):
    world = make_world()
    _make_open_plane(world)
    world.set_temperature(20.0)
    parent = _spawn_wildlife(world, 0, 0, "graze", 0.9)
    before = len(world.get_entities_with_component("Wildlife"))

    world.tick()

    after = len(world.get_entities_with_component("Wildlife"))
    assert after == before + 1
    wildlife = world.get_component(parent, "Wildlife")
    assert abs(wildlife["satiety"] - 0.45) < 1e-9
    assert wildlife["reproduction_cooldown"] == 10
    assert wildlife["state"] == "graze"
    children = [
        eid for eid in world.get_entities_with_component("Wildlife") if eid != parent
    ]
    assert len(children) == 1
    child_wildlife = world.get_component(children[0], "Wildlife")
    assert child_wildlife["state"] == "graze"
    assert abs(child_wildlife["satiety"] - 0.5) < 1e-9
    child_species = world.get_component(children[0], "Species")
    assert child_species["diet"] == "herbivore"
