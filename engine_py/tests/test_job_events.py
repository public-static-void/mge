import time

import pytest

# Persistent global list and callback to ensure the Python function is not garbage collected.
received_events = []


def persistent_on_job_completed(event):
    # This callback is used for job_completed event subscription.
    # It is global to ensure it is not garbage collected during the test.
    received_events.append(event)


def test_job_event_log_querying(make_world):
    world = make_world()
    # Create agents with skills so jobs can be assigned and completed.
    agent1 = world.spawn_entity()
    agent2 = world.spawn_entity()
    world.set_component(
        agent1, "Agent", {"entity_id": agent1, "skills": {"DigTunnel": 1.0}}
    )
    world.set_component(
        agent2, "Agent", {"entity_id": agent2, "skills": {"BuildWall": 1.0}}
    )

    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()

    # Assign jobs and trigger events.
    world.assign_job(
        eid1, "DigTunnel", state="pending", priority=5, category="test"
    )
    world.assign_job(
        eid2, "BuildWall", state="pending", priority=10, category="test"
    )

    # Run the job system enough times for assignment and completion.
    for _ in range(10):
        world.tick()

    # Query all job events.
    events = world.get_job_event_log()
    assert isinstance(events, list)
    # At least one job_assigned or job_completed event should be present.
    assert any(e["event_type"] == "job_assigned" for e in events) or any(
        e["event_type"] == "job_completed" for e in events
    )

    # Query by event type.
    assigned_events = world.get_job_events_by_type("job_assigned")
    assert isinstance(assigned_events, list)
    assert all(e["event_type"] == "job_assigned" for e in assigned_events)

    # Query since timestamp.
    now = int(time.time() * 1000)
    job3 = world.spawn_entity()
    world.assign_job(job3, "TestJob", state="pending", category="test")
    # Add an agent who can do TestJob.
    agent3 = world.spawn_entity()
    world.set_component(
        agent3, "Agent", {"entity_id": agent3, "skills": {"TestJob": 1.0}}
    )
    for _ in range(5):
        world.tick()
    events_since = world.get_job_events_since(now)
    assert isinstance(events_since, list)
    assert any(e["timestamp"] >= now for e in events_since)

    # Query by payload filter: jobs assigned to agent2.
    filtered = world.get_job_events_where(
        lambda payload: payload.get("assigned_to") == agent2
    )
    assert isinstance(filtered, list)
    for e in filtered:
        assert e["payload"]["assigned_to"] == agent2


def test_job_event_bus_polling(make_world):
    world = make_world()
    # Create agent so job can be assigned and completed.
    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"DigTunnel": 1.0}}
    )
    eid = world.spawn_entity()
    world.assign_job(eid, "DigTunnel", state="pending", category="test")
    for _ in range(5):
        world.tick()

    # Poll the job event bus for 'job_assigned' events.
    world.update_event_buses()
    events = world.poll_job_event_bus("job_assigned")
    assert isinstance(events, list)
    for e in events:
        assert e["event_type"] == "job_assigned"


def test_job_event_bus_subscription(make_world):
    world = make_world()
    # Create agent so job can be assigned and completed.
    agent = world.spawn_entity()
    world.set_component(
        agent, "Agent", {"entity_id": agent, "skills": {"DigTunnel": 1.0}}
    )
    eid = world.spawn_entity()
    global received_events
    received_events.clear()

    # Subscribe to job_completed events with a persistent callback.
    sub_id = world.subscribe_job_event_bus(
        "job_completed", persistent_on_job_completed
    )
    world.assign_job(eid, "DigTunnel", state="pending", category="test")
    for _ in range(10):
        world.tick()
        job_state = world.get_component(eid, "Job")["state"]
        if received_events:
            break

    assert received_events, "Should have received job_completed event"
    assert received_events[0]["event_type"] == "job_completed"

    # Unsubscribe and ensure no more events are received.
    world.unsubscribe_job_event_bus("job_completed", sub_id)
    received_events.clear()
    job2 = world.spawn_entity()
    agent2 = world.spawn_entity()
    world.set_component(
        agent2, "Agent", {"entity_id": agent2, "skills": {"DigTunnel": 1.0}}
    )
    world.assign_job(job2, "DigTunnel", state="pending", category="test")
    for _ in range(10):
        world.tick()
    assert not received_events, "Should not receive events after unsubscribe"
