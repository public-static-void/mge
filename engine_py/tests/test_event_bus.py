import json


def test_event_bus_send_and_poll(make_world):
    world = make_world()
    event_type = "test_event"
    world.send_event(event_type, json.dumps(42))  # Serialize as JSON string
    world.update_event_buses()
    events = world.poll_event(event_type)
    assert 42 in events


def test_event_bus_empty_poll(make_world):
    world = make_world()
    event_type = "test_event"
    # Send a dummy event to create the bus, then poll and clear it
    world.send_event(event_type, json.dumps(0))
    world.update_event_buses()
    world.poll_event(event_type)
    world.update_event_buses()  # <-- advance the bus again
    # Now poll again; should be empty
    events = world.poll_event(event_type)
    assert events == []
