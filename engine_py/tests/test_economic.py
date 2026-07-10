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


def test_enqueue_production_job(make_world):
    world = make_world()
    eid = world.spawn_entity()

    # First enqueue should return True
    result = world.enqueue_production_job(eid, "wood_plank", 1, 2)
    assert result is True, "First enqueue should return True"

    # Verify component was created
    job = world.get_production_queue(eid)
    assert job is not None
    assert job["recipe"] == "wood_plank"
    assert job["priority"] == 1
    assert job["batch_size"] == 2
    assert job["progress"] == 0
    assert job["state"] == "pending"

    # Second enqueue on same entity should return False
    result2 = world.enqueue_production_job(eid, "stone_bricks", 5, 1)
    assert result2 is False, "Second enqueue should return False"


def test_get_production_queue(make_world):
    world = make_world()
    eid = world.spawn_entity()

    # No job yet, should return None
    assert world.get_production_queue(eid) is None

    # Enqueue and verify
    world.enqueue_production_job(eid, "wood_plank", 3, 1)
    job = world.get_production_queue(eid)
    assert job is not None
    assert job["recipe"] == "wood_plank"
    assert job["priority"] == 3

    # Non-existent entity
    assert world.get_production_queue(99999) is None


def test_get_completed_production_jobs(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "Stockpile", {"resources": {"wood": 5}})

    # No completions yet
    completions = world.get_completed_production_jobs(eid)
    assert completions == []

    # Enqueue a job and run economic systems
    world.enqueue_production_job(eid, "wood_plank", 0, 1)
    world.run_native_system("EconomicSystem")
    world.run_native_system("EconomicSystem")

    # Update event buses
    world.update_event_buses()

    # Check completions
    completions = world.get_completed_production_jobs(eid)
    assert len(completions) >= 1
    assert completions[0]["recipe"] == "wood_plank"
