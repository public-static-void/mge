def test_spawn_and_set_component(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    comp = world.get_component(eid, "Health")
    print("Component:", comp)
