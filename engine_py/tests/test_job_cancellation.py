def test_cancel_job_marks_cancelled_and_filters_from_active(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(eid, "TestJob", state="pending", category="test")
    jobs = world.list_jobs()
    assert len(jobs) == 1
    job_id = jobs[0]["id"]

    world.cancel_job(job_id)
    job = world.get_job(job_id)
    assert job["cancelled"] is True

    for _ in range(3):
        world.tick()

    # By default, list_jobs() should not return cancelled jobs
    jobs_after = world.list_jobs()
    assert all(job["id"] != job_id for job in jobs_after), "Cancelled job should not be in active jobs"

    # But with include_terminal=True, it should appear
    jobs_with_terminal = world.list_jobs(include_terminal=True)
    assert any(job["id"] == job_id for job in jobs_with_terminal), "Cancelled job should be available for introspection"
