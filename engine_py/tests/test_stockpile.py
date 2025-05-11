def test_modify_stockpile_resource(make_world):
    world = make_world()
    entity = world.spawn_entity()
    world.set_component(entity, "Stockpile", {"resources": {"wood": 10, "stone": 5}})

    # Add 3 food
    world.modify_stockpile_resource(entity, "food", 3)
    # Remove 2 wood
    world.modify_stockpile_resource(entity, "wood", -2)

    stockpile = world.get_component(entity, "Stockpile")
    assert stockpile["resources"]["wood"] == 8
    assert stockpile["resources"]["stone"] == 5
    assert stockpile["resources"]["food"] == 3

    import pytest
    with pytest.raises(ValueError):
        world.modify_stockpile_resource(entity, "stone", -10)
