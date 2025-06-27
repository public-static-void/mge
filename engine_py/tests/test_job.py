def test_job_completion(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(eid, "test_job", category="testing", state="pending")
    found = False
    for _ in range(12):  # enough ticks to guarantee completion
        world.run_native_system("JobSystem")
        world.update_event_buses()
        events = world.poll_ecs_event("job_completed")
        print("Events after tick:", events)
        if any(e["entity"] == eid for e in events):
            found = True
            break
    assert found


def test_get_job_types(make_world):
    world = make_world()

    # Register a custom job type if needed for the test
    def dummy_job_logic(job, progress):
        job["state"] = "complete"
        return job

    world.register_job_type("DummyJob", dummy_job_logic)
    types = world.get_job_types()
    assert isinstance(types, list)
    assert "DummyJob" in types
