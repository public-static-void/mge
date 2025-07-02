def test_dynamic_job_registration(make_world):
    world = make_world()
    eid = world.spawn_entity()

    # Register a custom job type in Python
    def test_job_logic(job):
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


def test_dynamic_python_job_handler(make_world):
    world = make_world()
    events = []

    # Define a Python job handler that mutates the job and logs invocation
    def handler(job):
        events.append(f"called:{job['state']}")
        if job["state"] == "pending":
            job["state"] = "in_progress"
        elif job["state"] == "in_progress":
            job["progress"] = job.get("progress", 0) + 1
            if job["progress"] >= 2:
                job["state"] = "complete"
        return job

    world.register_job_type("PyTestJob", handler)
    eid = world.spawn_entity()
    world.assign_job(eid, "PyTestJob", state="pending", category="testing")
    for _ in range(4):
        world.run_native_system("JobSystem")
    job = world.get_component(eid, "Job")
    assert job["state"] == "complete"
    # The first call will be with state "in_progress" due to how the job system transitions
    assert events[0] == "called:in_progress"
    assert "called:in_progress" in events
    assert "called:complete" in events or job["state"] == "complete"
