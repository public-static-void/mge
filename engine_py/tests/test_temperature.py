def test_get_temperature_returns_default(make_world):
    world = make_world()
    assert world.get_temperature() == 15.0


def test_set_temperature_round_trip(make_world):
    world = make_world()
    world.set_temperature(20.0)
    assert world.get_temperature() == 20.0


def test_set_temperature_clamps_high(make_world):
    world = make_world()
    world.set_temperature(100.0)
    assert world.get_temperature() == 60.0


def test_set_temperature_clamps_low(make_world):
    world = make_world()
    world.set_temperature(-100.0)
    assert world.get_temperature() == -60.0


def test_override_holds_across_tick(make_world):
    world = make_world()
    world.set_temperature(20.0)
    world.tick()
    assert world.get_temperature() == 20.0


def test_set_temperature_emits_changed_event(make_world):
    world = make_world()
    world.set_temperature(20.0)
    world.update_event_buses()
    events = world.poll_ecs_event("temperature_changed")
    assert len(events) == 1
    assert events[0]["old_ambient"] == 15.0
    assert events[0]["new_ambient"] == 20.0


def test_natural_drift_emits_no_changed_event(make_world):
    world = make_world()
    world.tick()
    world.update_event_buses()
    events = world.poll_ecs_event("temperature_changed")
    assert events == []


def test_body_exchange_drifts_toward_ambient(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_component(
        e,
        "Body",
        {
            "parts": [
                {
                    "name": "torso",
                    "status": "healthy",
                    "kind": "flesh",
                    "temperature": 37.0,
                    "ideal_temperature": 37.0,
                    "insulation": 0.0,
                    "heat_loss": 0.0,
                    "children": [],
                    "equipped": [],
                }
            ]
        },
    )
    world.set_temperature(0.0)
    world.tick()
    body = world.get_component(e, "Body")
    torso = body["parts"][0]
    assert abs(torso["temperature"] - 35.15) < 0.0001
    assert abs(torso["heat_loss"] - 1.85) < 0.0001


def test_humidity_pressure_round_trip(make_world):
    world = make_world()
    assert world.get_humidity() == 0.5
    assert world.get_pressure() == 1013.0
    world.set_humidity(0.8)
    assert world.get_humidity() == 0.8
    world.set_pressure(1000.0)
    assert world.get_pressure() == 1000.0


def test_humidity_pressure_clamps(make_world):
    world = make_world()
    world.set_humidity(2.0)
    assert world.get_humidity() == 1.0
    world.set_humidity(-1.0)
    assert world.get_humidity() == 0.0
    world.set_pressure(2000.0)
    assert world.get_pressure() == 1100.0
    world.set_pressure(500.0)
    assert world.get_pressure() == 900.0


def test_humidity_pressure_rejects_non_finite(make_world):
    world = make_world()
    world.set_humidity(0.7)
    world.set_pressure(1000.0)
    world.set_humidity(float("nan"))
    world.set_pressure(float("inf"))
    assert world.get_humidity() == 0.7
    assert world.get_pressure() == 1000.0


def test_humidity_pressure_shift_ambient(make_world):
    world = make_world()
    world.set_weather("Clear", 0.0, 100)
    world.set_humidity(0.5)
    world.set_pressure(1013.0)
    world.tick()
    neutral = world.get_temperature()
    world.set_humidity(1.0)
    world.set_pressure(1013.0)
    world.tick()
    assert abs((world.get_temperature() - neutral) - 3.0) < 0.5
    world.set_humidity(0.5)
    world.set_pressure(973.0)
    world.tick()
    assert abs((world.get_temperature() - neutral) + 2.0) < 0.5
