import os

import mge


def make_world():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    return mge.PyWorld(schema_dir)


def test_move_and_damage():
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"x": 0, "y": 0})
    world.move_entity(eid, 2, 3)
    pos = world.get_component(eid, "Position")
    assert pos["x"] == 2 and pos["y"] == 3

    eid2 = world.spawn_entity()
    world.set_component(eid2, "Position", {"x": 1, "y": 1})
    world.move_all(1, 1)
    pos1 = world.get_component(eid, "Position")
    pos2 = world.get_component(eid2, "Position")
    assert pos1["x"] == 3 and pos1["y"] == 4
    assert pos2["x"] == 2 and pos2["y"] == 2


def test_damage_and_tick():
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    world.damage_entity(eid, 3)
    health = world.get_component(eid, "Health")
    assert health["current"] == 7

    eid2 = world.spawn_entity()
    world.set_component(eid2, "Health", {"current": 5, "max": 5})
    world.damage_all(2)
    h1 = world.get_component(eid, "Health")
    h2 = world.get_component(eid2, "Health")
    assert h1["current"] == 5
    assert h2["current"] == 3


def test_tick_and_turn():
    world = make_world()
    assert world.get_turn() == 0
    world.tick()
    assert world.get_turn() == 1


def test_process_deaths_and_decay():
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Health", {"current": 0, "max": 10})
    world.process_deaths()
    corpse = world.get_component(eid, "Corpse")
    decay = world.get_component(eid, "Decay")
    assert corpse is not None and decay is not None
    world.process_decay()
    # Should remove entity if decay reaches 0 (simulate ticks)
    for _ in range(5):
        world.process_decay()
    # Entity should be gone from all component maps
    for comp in ["Corpse", "Decay"]:
        assert world.get_component(eid, comp) is None


def test_count_entities_with_type():
    world = make_world()
    e1 = world.spawn_entity()
    e2 = world.spawn_entity()
    world.set_component(e1, "Type", {"kind": "player"})
    world.set_component(e2, "Type", {"kind": "enemy"})
    assert world.count_entities_with_type("player") == 1
    assert world.count_entities_with_type("enemy") == 1
