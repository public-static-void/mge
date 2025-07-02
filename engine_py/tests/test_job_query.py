def test_list_jobs(make_world):
    world = make_world()
    e1 = world.spawn_entity()
    e2 = world.spawn_entity()
    world.assign_job(e1, "TestJob", state="pending", category="test")
    world.assign_job(e2, "TestJob", state="in_progress", category="test")
    jobs = world.list_jobs()
    assert isinstance(jobs, list)
    job_states = sorted(j["state"] for j in jobs)
    assert job_states == ["in_progress", "pending"]


def test_get_job_by_id(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(eid, "TestJob", state="pending", category="test")
    jobs = world.list_jobs()
    assert jobs
    job_id = jobs[0]["id"]
    job = world.get_job(job_id)
    assert isinstance(job, dict)
    assert job["state"] == "pending"
    assert job["job_type"] == "TestJob"


def test_find_jobs_by_state_and_type(make_world):
    world = make_world()
    e1 = world.spawn_entity()
    e2 = world.spawn_entity()
    world.assign_job(e1, "TestJob", state="pending", category="test")
    world.assign_job(e2, "OtherJob", state="in_progress", category="test")
    pending_jobs = world.find_jobs(state="pending")
    assert isinstance(pending_jobs, list)
    assert len(pending_jobs) == 1
    assert pending_jobs[0]["job_type"] == "TestJob"
    other_jobs = world.find_jobs(job_type="OtherJob")
    assert isinstance(other_jobs, list)
    assert len(other_jobs) == 1
    assert other_jobs[0]["state"] == "in_progress"
