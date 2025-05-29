def test_entities_in_cell(make_world):
    world = make_world()
    world.add_cell(0, 0, 0)
    eid = world.spawn_entity()
    world.set_component(
        eid, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}
    )
    cell = {"Square": {"x": 0, "y": 0, "z": 0}}
    entities = world.entities_in_cell(cell)
    assert len(entities) == 1
    assert entities[0] == eid
