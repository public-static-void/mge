def test_advance_job_progress(make_world):
    world = make_world()
    eid = world.spawn_entity()

    agent_id = world.spawn_entity()
    world.set_component(agent_id, "Agent", {"entity_id": agent_id, "skills": {"TestJob": 1.0}})

    world.assign_job(
        eid,
        "TestJob",
        state="pending",
        progress=0.0,
        category="test",
        assigned_to=agent_id,
        target=None,
        reserved_stockpile=None,
        target_position=None,
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


def test_skill_multiplier_regression(make_world):
    world = make_world()

    # Agent A: high skill (5.0) in TestJob via SkillLevels
    agent_a = world.spawn_entity()
    world.set_component(agent_a, "SkillLevels", {
        "skills": {"TestJob": 5.0},
        "skill_levels": {"TestJob": 5.0},
        "total_xp": 120.0,
        "skill_xp": {"TestJob": 120.0}
    })
    world.set_component(agent_a, "Agent", {
        "entity_id": agent_a,
        "stamina": 100.0,
        "state": "working"
    })

    # Agent B: low skill (1.0) in TestJob via SkillLevels
    agent_b = world.spawn_entity()
    world.set_component(agent_b, "SkillLevels", {
        "skills": {"TestJob": 1.0},
        "skill_levels": {"TestJob": 1.0},
        "total_xp": 0.0,
        "skill_xp": {"TestJob": 0.0}
    })
    world.set_component(agent_b, "Agent", {
        "entity_id": agent_b,
        "stamina": 100.0,
        "state": "working"
    })

    # Create two jobs of the same type with high required_progress
    job_a_id = world.spawn_entity()
    world.assign_job(
        job_a_id,
        "TestJob",
        state="in_progress",
        progress=0.0,
        required_progress=100.0,
        category="test",
        assigned_to=agent_a,
    )

    job_b_id = world.spawn_entity()
    world.assign_job(
        job_b_id,
        "TestJob",
        state="in_progress",
        progress=0.0,
        required_progress=100.0,
        category="test",
        assigned_to=agent_b,
    )

    # Advance both jobs once
    world.advance_job_state(job_a_id)
    world.advance_job_state(job_b_id)

    job_a = world.get_job(job_a_id)
    job_b = world.get_job(job_b_id)

    # skill=5.0 => increment = 1.0 * 5.0 * (100/100) = 5.0
    # skill=1.0 => increment = 1.0 * 1.0 * (100/100) = 1.0
    # Verify: progress_skill5 > progress_skill1 * 4
    assert job_a["progress"] > job_b["progress"] * 4, (
        f"Expected skill=5 progress > 4x skill=1 progress, "
        f"got {job_a['progress']} vs {job_b['progress']}"
    )
