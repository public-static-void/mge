def test_get_entities_with_component(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()
    world.set_component(eid1, "Health", {"current": 5, "max": 10})
    ids = world.get_entities_with_component("Health")
    assert eid1 in ids
    assert eid2 not in ids


def test_get_entities(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()
    all_ids = world.get_entities()
    assert eid1 in all_ids
    assert eid2 in all_ids


def test_get_entities_with_components(make_world):
    w = make_world()
    e1 = w.spawn_entity()
    e2 = w.spawn_entity()
    e3 = w.spawn_entity()
    w.set_component(e1, "Health", {"current": 10, "max": 10})
    w.set_component(e1, "Position", {"pos": {"Square": {"x": 1, "y": 2, "z": 0}}})
    w.set_component(e2, "Health", {"current": 5, "max": 10})
    w.set_component(e3, "Position", {"pos": {"Square": {"x": 3, "y": 4, "z": 0}}})
    both = w.get_entities_with_components(["Health", "Position"])
    assert both == [e1]
