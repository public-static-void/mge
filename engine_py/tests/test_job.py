import pytest


def test_job_completion(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.assign_job(eid, "test_job")
    found = False
    for _ in range(12):  # enough ticks to guarantee completion
        world.run_native_system("JobSystem")
        world.update_event_buses()
        events = world.poll_ecs_event("job_completed")
        print("Events after tick:", events)
        if any(e["entity"] == eid for e in events):
            found = True
            break
    assert found
