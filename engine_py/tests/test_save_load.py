def test_save_and_load(make_world, tmp_path):
    world = make_world()
    world.set_mode("roguelike")
    e = world.spawn_entity()
    world.set_component(e, "Health", {"current": 99, "max": 100})

    save_file = tmp_path / "test_save.json"
    world.save_to_file(str(save_file))

    world.despawn_entity(e)
    assert len(world.get_entities()) == 0

    world.load_from_file(str(save_file))
    entities = world.get_entities()
    assert len(entities) > 0
    h = world.get_component(entities[0], "Health")
    assert h["current"] == 99
