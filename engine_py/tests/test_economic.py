def test_economic_helpers(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Stockpile", {"resources": {"wood": 5}})
    world.set_component(eid, "ProductionJob", {
        "recipe": "wood_plank",
        "progress": 0,
        "state": "pending"
    })

    # Test get_stockpile_resources
    resources = world.get_stockpile_resources(eid)
    assert resources is not None
    assert resources["wood"] == 5

    # Test get_production_job
    job = world.get_production_job(eid)
    assert job is not None
    assert job["recipe"] == "wood_plank"
    assert job["state"] == "pending"

    # Remove and test None
    world.remove_component(eid, "Stockpile")
    assert world.get_stockpile_resources(eid) is None

    world.remove_component(eid, "ProductionJob")
    assert world.get_production_job(eid) is None
