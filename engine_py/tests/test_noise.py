"""Tests for the noise and detection system via the Python API."""


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


def _set_position(world, eid, x, y, z):
    world.set_component(eid, "Position", {"pos": {"Square": {"x": x, "y": y, "z": z}}})


def test_emit_noise_get_noise_at(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = world.spawn_entity()
    _set_position(world, eid, 0, 0, 0)
    assert world.emit_noise(eid, 1.0, 5) is True

    world.tick()

    assert world.get_noise_at(0, 0, 0) == 1.0
    assert world.get_noise_at(1, 0, 0) == 0.8
    assert world.get_noise_at(2, 0, 0) == 0.6
    assert world.get_noise_at(6, 0, 0) == 0.0


def test_hearing_detection(make_world):
    world = make_world()
    _make_open_plane(world)
    enemy = world.spawn_entity()
    world.set_component(
        enemy,
        "EnemyAI",
        {
            "state": "idle",
            "alert_level": 0,
            "detection_range": 8,
            "attack_range": 1,
            "flee_threshold": 0.25,
            "target_faction": None,
            "target_entity": None,
        },
    )
    _set_position(world, enemy, 1, 0, 0)
    assert world.set_hearing(enemy, 5, 1.0) is True

    emitter = world.spawn_entity()
    _set_position(world, emitter, 0, 0, 0)
    world.emit_noise(emitter, 0.5, 5)

    world.tick()

    ai = world.get_component(enemy, "EnemyAI")
    assert ai is not None
    assert ai["alert_level"] > 0, "alert_level should increase after hearing noise"
    assert ai["state"] == "investigate", "Enemy should investigate after hearing noise"


def test_stealth_modifier(make_world):
    world = make_world()
    _make_open_plane(world)
    eid = world.spawn_entity()
    _set_position(world, eid, 0, 0, 0)
    world.emit_noise(eid, 1.0, 5)
    world.set_component(eid, "Stealth", {"noise_modifier": 0.5})

    world.tick()

    assert world.get_noise_at(0, 0, 0) == 0.5
    assert world.get_noise_at(1, 0, 0) == 0.4


def test_get_hearing_roundtrip(make_world):
    world = make_world()
    eid = world.spawn_entity()
    assert world.get_hearing(eid) is None
    assert world.set_hearing(eid, 7, 1.5) is True
    hearing = world.get_hearing(eid)
    assert hearing is not None
    assert hearing["range"] == 7
    assert hearing["sensitivity"] == 1.5