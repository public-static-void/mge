def test_set_job_field(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(
        eid, "TestJob", state="pending", progress=0.0, category="test"
    )
    jobs = world.list_jobs()
    job_id = jobs[0]["id"]
    world.set_job_field(job_id, "state", "in_progress")
    job = world.get_job(job_id)
    assert job["state"] == "in_progress"
    world.set_job_field(job_id, "progress", 0.5)
    job = world.get_job(job_id)
    assert job["progress"] == 0.5


def test_update_job(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(
        eid, "TestJob", state="pending", progress=0.0, category="test"
    )
    jobs = world.list_jobs()
    job_id = jobs[0]["id"]
    world.update_job(job_id, state="complete", progress=1.0, custom="foo")
    job = world.get_job(job_id)
    assert job["state"] == "complete"
    assert job["progress"] == 1.0
    assert job["custom"] == "foo"
