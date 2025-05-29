def test_move_and_damage(make_world):
    world = make_world()

    # Add cells to the map after world is created
    world.add_cell(0, 0, 0)
    world.add_cell(1, 1, 0)
    world.add_cell(2, 3, 0)
    world.add_cell(3, 4, 0)
    world.add_cell(2, 2, 0)

    eid = world.spawn_entity()
    world.set_component(
        eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}
    )
    world.move_entity(eid, 2, 3)
    print("After move_entity:", world.get_component(eid, "Position"))

    eid2 = world.spawn_entity()
    world.set_component(
        eid2,
        "Position",
        {"pos": {"Square": {"x": 1, "y": 1, "z": 0}}},
    )
    world.move_all(1, 1)
    print(
        "After move_all (eid):", world.get_component(eid, "Position")
    )
    print(
        "After move_all (eid2):",
        world.get_component(eid2, "Position"),
    )

    pos1 = world.get_component(eid, "Position")
    pos2 = world.get_component(eid2, "Position")
    assert pos1["pos"]["Square"]["x"] == 3 and pos1["pos"]["Square"]["y"] == 4
    assert pos2["pos"]["Square"]["x"] == 2 and pos2["pos"]["Square"]["y"] == 2


def test_damage_and_tick(make_world):
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


def test_tick_and_turn(make_world):
    world = make_world()
    assert world.get_turn() == 0
    world.tick()
    assert world.get_turn() == 1


def test_process_deaths_and_decay(make_world):
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


def test_count_entities_with_type(make_world):
    world = make_world()
    e1 = world.spawn_entity()
    e2 = world.spawn_entity()
    world.set_component(e1, "Type", {"kind": "player"})
    world.set_component(e2, "Type", {"kind": "enemy"})
    assert world.count_entities_with_type("player") == 1
    assert world.count_entities_with_type("enemy") == 1
