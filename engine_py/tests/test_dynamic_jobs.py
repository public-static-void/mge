def test_dynamic_job_registration(make_world):
    world = make_world()
    eid = world.spawn_entity()

    # Register a custom job type in Python
    def test_job_logic(job, progress):
        if job["state"] == "pending":
            job["state"] = "in_progress"
            job["progress"] = 0
        elif job["state"] == "in_progress":
            job["progress"] = job.get("progress", 0) + 1
            if job["progress"] >= 2:
                job["state"] = "complete"
        return job

    world.register_job_type("TestJob", test_job_logic)
    world.assign_job(eid, "TestJob", category="testing", state="pending")

    # Run the job system a few times
    for _ in range(4):
        world.run_native_system("JobSystem")

    # Check that the job is marked complete
    job = world.get_component(eid, "Job")
    assert job["state"] == "complete"


def test_dynamic_system_registration(make_world):
    world = make_world()
    called = {"flag": False}

    def my_system(dt):
        called["flag"] = True

    world.register_system("TestSystem", my_system)
    world.run_system("TestSystem")
    assert called["flag"]
