def test_advance_job_progress(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(
        eid, "TestJob", state="pending", progress=0.0, category="test"
    )

    jobs = world.list_jobs()
    assert len(jobs) == 1
    job_id = jobs[0]["id"]

    job = world.get_job(job_id)
    assert job["state"] == "pending"
    assert job["progress"] == 0.0

    # Advance the job once
    world.advance_job_state(job_id)

    job = world.get_job(job_id)
    assert job["state"] in ("in_progress", "pending")
    assert job["progress"] > 0.0

    # Advance multiple times to complete the job
    for _ in range(10):
        world.advance_job_state(job_id)

    job = world.get_job(job_id)
    assert job["state"] == "complete"
    assert job["progress"] >= 3.0

    # Advancing a completed job should not change state or progress
    prev_progress = job["progress"]
    world.advance_job_state(job_id)
    job = world.get_job(job_id)
    assert job["state"] == "complete"
    assert job["progress"] == prev_progress
