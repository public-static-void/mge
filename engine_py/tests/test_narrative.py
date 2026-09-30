"""Bridge parity tests for the event-driven narrative engine (incident director).

Covers the identical seven-function surface through the Python bridge:
register/list/get, poll/get-pending, resolve, history. Trigger evaluation
stays in Rust core; the bridge only registers, polls, and resolves.
"""

import json

import pytest


def turn_gate_def(scenario_id):
    return json.dumps(
        {
            "id": scenario_id,
            "name": f"Scenario {scenario_id}",
            "triggers": [{"type": "turn_gte", "turn": 1}],
            "choices": [
                {
                    "id": "take",
                    "label": "Take",
                    "effects": [
                        {
                            "action": "emit_event",
                            "data": {
                                "bus": "narr_test_bus",
                                "payload": {"note": "picked"},
                            },
                        }
                    ],
                }
            ],
        }
    )


def test_register_list_get_roundtrip(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_alpha"))
    listed = world.list_scenarios()
    assert len(listed) == 1
    assert listed[0]["id"] == "py_alpha"
    one = world.get_scenario("py_alpha")
    assert one["name"] == "Scenario py_alpha"
    assert world.get_scenario("missing") is None


def test_duplicate_registration_replaces(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_dup"))
    world.register_scenario(turn_gate_def("py_dup"))
    assert len(world.list_scenarios()) == 1
    world.tick()
    assert len(world.poll_pending_decisions()) == 1


def test_malformed_registration_errors(make_world):
    world = make_world()
    with pytest.raises(ValueError, match="id"):
        world.register_scenario('{"id":"","name":"bad","choices":[]}')
    assert world.list_scenarios() == []


def test_tick_fires_pending_decision(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_fire"))
    assert world.poll_pending_decisions() == []
    world.tick()
    pending = world.poll_pending_decisions()
    assert len(pending) == 1
    assert pending[0]["scenario_id"] == "py_fire"
    world.tick()
    assert len(world.poll_pending_decisions()) == 1


def test_get_pending_decision_lookup(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_lookup"))
    world.tick()
    pending = world.poll_pending_decisions()
    one = world.get_pending_decision(pending[0]["id"])
    assert one["scenario_id"] == "py_lookup"
    assert world.get_pending_decision(9999) is None


def test_resolve_applies_effect_and_history(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_resolve"))
    world.tick()
    pending = world.poll_pending_decisions()
    world.resolve_decision(pending[0]["id"], "take")
    assert world.poll_pending_decisions() == []
    # Direct resolves bypass the tick buffer swap, so flush before polling.
    world.update_event_buses()
    events = world.poll_ecs_event("narr_test_bus")
    assert len(events) == 1
    assert events[0]["note"] == "picked"
    history = world.get_narrative_history()
    assert len(history) == 2
    assert history[0]["kind"] == "fired"
    assert history[1]["kind"] == "resolved"
    assert history[1]["choice_id"] == "take"


def test_unknown_resolve_errors(make_world):
    world = make_world()
    world.register_scenario(turn_gate_def("py_err"))
    world.tick()
    with pytest.raises(ValueError, match="9999"):
        world.resolve_decision(9999, "take")
    pending = world.poll_pending_decisions()
    with pytest.raises(ValueError, match="ghost"):
        world.resolve_decision(pending[0]["id"], "ghost")
    assert len(world.poll_pending_decisions()) == 1
    assert len(world.get_narrative_history()) == 1
