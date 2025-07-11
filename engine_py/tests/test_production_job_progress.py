def test_production_job_progress_and_state(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_component(
        e,
        "ProductionJob",
        {
            "state": "pending",
            "progress": 0,
            "recipe": "wood_plank",
        },
    )

    progress = world.get_production_job_progress(e)
    assert progress == 0
    state = world.get_production_job_state(e)
    assert state == "pending"

    world.set_production_job_state(e, "in_progress")
    world.set_production_job_progress(e, 2)
    new_progress = world.get_production_job_progress(e)
    assert new_progress == 2
    new_state = world.get_production_job_state(e)
    assert new_state == "in_progress"
