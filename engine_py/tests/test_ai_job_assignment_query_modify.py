import pytest


def test_query_ai_jobs_empty(make_world):
    world = make_world()
    # Agent with skill but no assigned jobs yet
    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"TestJob": 1.0}}
    )

    assigned_jobs = world.ai_query_jobs(agent)
    assert isinstance(assigned_jobs, list)
    assert len(assigned_jobs) == 0


def test_query_ai_jobs_after_assignment(make_world):
    world = make_world()
    agent = world.spawn_entity()
    world.set_component(
        agent,
        "Agent",
        {
            "entity_id": agent,
            "skills": {"TestJob": 1.0},
            "specializations": ["test"],
            "state": "idle",
            "current_job": None,
        },
    )

    job_entity = world.spawn_entity()
    world.assign_job(job_entity, "TestJob", state="pending", category="test")

    world.ai_assign_jobs(agent, [])

    agent_comp = world.get_component(agent, "Agent")
    job_comp = world.get_component(job_entity, "Job")
    assigned_jobs = world.ai_query_jobs(agent)
    assert any(j["id"] == job_entity for j in assigned_jobs)


def test_modify_ai_job_assignment_valid(make_world):
    world = make_world()
    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"TestJob": 1.0}}
    )

    job_entity = world.spawn_entity()
    world.assign_job(job_entity, "TestJob", state="pending", category="test")
    world.ai_assign_jobs(agent, [])

    success = world.ai_modify_job_assignment(job_entity, assigned_to=None)
    assert success is True

    assigned_jobs = world.ai_query_jobs(agent)
    assert all(j["id"] != job_entity for j in assigned_jobs)


def test_modify_ai_job_assignment_invalid(make_world):
    world = make_world()
    invalid_job_id = 999999

    with pytest.raises(Exception) as excinfo:
        world.ai_modify_job_assignment(invalid_job_id, assigned_to=0)

    assert "No job with id" in str(excinfo.value)
