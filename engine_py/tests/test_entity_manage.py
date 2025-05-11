def test_despawn_and_remove_component(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    world.remove_component(eid, "Health")
    assert world.get_component(eid, "Health") is None
    world.despawn_entity(eid)
    assert eid not in world.get_entities()


def test_is_entity_alive(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    assert world.is_entity_alive(eid)
    world.set_component(eid, "Health", {"current": 0, "max": 10})
    assert not world.is_entity_alive(eid)
