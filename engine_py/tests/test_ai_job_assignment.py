import pytest


def test_assign_jobs_basic(make_world):
    world = make_world()
    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"TestJob": 1.0}}
    )

    job_entity = world.spawn_entity()
    world.assign_job(
        job_entity,
        "TestJob",
        state="pending",
        category="test",
        assigned_to=None,
    )

    jobs_before = world.list_jobs()
    assert any(
        j["id"] == job_entity
        and (not j.get("assigned_to") or j.get("assigned_to") == 0)
        for j in jobs_before
    )

    world.ai_assign_jobs(agent, [])

    jobs_after = world.list_jobs()
    assigned_jobs = [j for j in jobs_after if j.get("assigned_to") == agent]
    assert any(
        j["id"] == job_entity for j in assigned_jobs
    ), "Job should be assigned to agent"


def test_assign_jobs_multiple_agents(make_world):
    world = make_world()

    agent1 = world.spawn_entity()
    world.set_component(
        agent1, "Agent", {"entity_id": agent1, "skills": {"TestJob": 1.0}}
    )

    agent2 = world.spawn_entity()
    world.set_component(
        agent2, "Agent", {"entity_id": agent2, "skills": {"TestJob": 1.0}}
    )

    job1 = world.spawn_entity()
    world.assign_job(job1, "TestJob", state="pending", category="test")

    job2 = world.spawn_entity()
    world.assign_job(job2, "TestJob", state="pending", category="test")

    world.ai_assign_jobs(agent1, [])
    world.ai_assign_jobs(agent2, [])

    jobs = world.list_jobs()
    assigned_jobs = [
        j for j in jobs if j["state"] in ("pending", "in_progress")
    ]
    assert len(assigned_jobs) == 2

    assigned_counts = {}
    for j in assigned_jobs:
        assigned_to = j.get("assigned_to")
        assigned_counts[assigned_to] = assigned_counts.get(assigned_to, 0) + 1

    count1 = assigned_counts.get(agent1, 0)
    count2 = assigned_counts.get(agent2, 0)
    diff = abs(count1 - count2)
    assert diff <= 1


def test_assign_jobs_no_jobs(make_world):
    world = make_world()

    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"TestJob": 1.0}}
    )

    # No jobs created here
    world.ai_assign_jobs(agent, [])

    jobs = world.list_jobs()
    assert len(jobs) == 0
