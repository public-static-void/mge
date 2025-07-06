def test_get_job_board(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()
    world.assign_job(
        eid1, "JobA", state="pending", priority=5, category="test"
    )
    world.assign_job(
        eid2, "JobB", state="pending", priority=10, category="test"
    )

    jobs = world.get_job_board()
    assert isinstance(jobs, list)
    # Should contain both jobs, sorted by priority (descending)
    assert len(jobs) == 2
    assert jobs[0]["eid"] == eid2
    assert jobs[1]["eid"] == eid1
    assert jobs[0]["priority"] == 10
    assert jobs[1]["priority"] == 5
    assert jobs[0]["state"] == "pending"
    assert jobs[1]["state"] == "pending"


def test_job_board_policy_and_priority(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()
    eid3 = world.spawn_entity()
    world.assign_job(
        eid1, "JobA", state="pending", priority=5, category="test"
    )
    world.assign_job(
        eid2, "JobB", state="pending", priority=10, category="test"
    )
    world.assign_job(
        eid3, "JobC", state="pending", priority=1, category="test"
    )

    # Default policy is "priority"
    assert world.get_job_board_policy() == "priority"
    jobs = world.get_job_board()
    assert [j["eid"] for j in jobs] == [eid2, eid1, eid3]

    # Change to FIFO
    world.set_job_board_policy("fifo")
    assert world.get_job_board_policy() == "fifo"
    jobs = world.get_job_board()
    assert [j["eid"] for j in jobs] == [eid1, eid2, eid3]

    # Change to LIFO
    world.set_job_board_policy("lifo")
    assert world.get_job_board_policy() == "lifo"
    jobs = world.get_job_board()
    assert [j["eid"] for j in jobs] == [eid3, eid2, eid1]

    # Test get/set job priority
    assert world.get_job_priority(eid1) == 5
    world.set_job_priority(eid1, 42)
    assert world.get_job_priority(eid1) == 42
