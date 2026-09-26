"""Parity tests for Vehicle support (ROADMAP L56) via the Python API."""

import pytest


def _make_open_plane(world, size=10):
    for x in range(-size, size + 1):
        for y in range(-size, size + 1):
            world.add_cell(x, y, 0)
    for x in range(-size, size + 1):
        for y in range(-size, size + 1):
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if -size <= nx <= size and -size <= ny <= size:
                        world.add_neighbor((x, y, 0), (nx, ny, 0))


def _make_corridor(world, length=4):
    for x in range(length + 1):
        world.add_cell(x, 0, 0)
    for x in range(length):
        world.add_neighbor((x, 0, 0), (x + 1, 0, 0))
        world.add_neighbor((x + 1, 0, 0), (x, 0, 0))


def _square_cell(x, y):
    return {"Square": {"x": x, "y": y, "z": 0}}


def _spawn_vehicle(world, x, y, capacity=2, speed=1, blocked=None):
    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"pos": _square_cell(x, y)})
    world.set_component(
        eid,
        "Vehicle",
        {"capacity": capacity, "speed": speed, "blocked_terrains": blocked or []},
    )
    return eid


def _spawn_rider(world, x, y):
    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"pos": _square_cell(x, y)})
    world.set_component(eid, "Agent", {"entity_id": eid})
    return eid


def _pos_of(world, eid):
    return world.get_component(eid, "Position")["pos"]


def test_vehicle_schema_registered_with_defaults(make_world):
    world = make_world()
    schema = world.get_component_schema("Vehicle")
    assert schema is not None, "Vehicle schema must be retrievable"
    assert schema["title"] == "Vehicle", "schema title must be Vehicle"

    eid = world.spawn_entity()
    world.set_component(eid, "Vehicle", {"capacity": 2, "speed": 1})
    stored = world.get_component(eid, "Vehicle")
    assert stored["capacity"] == 2, "capacity should round-trip"
    assert stored["speed"] == 1, "speed should round-trip"


def test_embark_records_occupancy_clears_path_and_emits(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider = _spawn_rider(world, 0, 0)
    world.set_component(
        rider, "Agent", {"entity_id": rider, "move_path": [_square_cell(1, 0)]}
    )

    ok, err = world.embark_vehicle(vehicle, rider)
    assert ok is True, "embark should succeed"
    assert err is None, "err should be None on success"

    occupants = world.get_vehicle_occupants(vehicle)
    assert occupants == [rider], "rider should be listed"
    assert world.is_mounted(rider) is True, "rider should be mounted"
    agent = world.get_component(rider, "Agent")
    assert agent.get("move_path") in (None, []), "embark must clear the rider path"

    world.update_event_buses()
    events = world.poll_ecs_event("vehicle_embarked")
    assert len(events) == 1, "one vehicle_embarked event expected"
    assert events[0]["vehicle"] == vehicle, "event vehicle mismatch"
    assert events[0]["rider"] == rider, "event rider mismatch"


def test_embark_rejects_bad_ids_and_double_mount(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    other = _spawn_vehicle(world, 1, 0)
    rider = _spawn_rider(world, 0, 0)

    ok, err = world.embark_vehicle(9999, rider)
    assert ok is False, "unknown vehicle should fail"
    assert err == "no_vehicle", "err should be no_vehicle"

    ok, err = world.embark_vehicle(vehicle, 9999)
    assert ok is False, "unknown rider should fail"
    assert err == "no_rider", "err should be no_rider"

    ok, err = world.embark_vehicle(vehicle, other)
    assert ok is False, "vehicle-as-rider should fail"
    assert err == "no_rider", "err should be no_rider"

    ok, _ = world.embark_vehicle(vehicle, rider)
    assert ok is True, "first embark should succeed"
    ok, err = world.embark_vehicle(other, rider)
    assert ok is False, "double mount should fail"
    assert err == "already_mounted", "err should be already_mounted"
    assert world.get_vehicle_occupants(other) == [], "no transfer on double mount"


def test_disembark_colocates_rider_and_emits(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider)
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [rider],
            "move_path": [_square_cell(2, 0)],
        },
    )
    world.tick()

    ok, err = world.disembark_vehicle(rider)
    assert ok is True, "disembark should succeed"
    assert err is None, "err should be None on success"
    assert world.is_mounted(rider) is False, "rider should no longer be mounted"
    assert world.get_vehicle_occupants(vehicle) == [], "occupants should be empty"
    vpos = _pos_of(world, vehicle)
    rpos = _pos_of(world, rider)
    assert rpos["Square"]["x"] == vpos["Square"]["x"], "rider x must equal vehicle x"
    assert rpos["Square"]["y"] == vpos["Square"]["y"], "rider y must equal vehicle y"

    world.update_event_buses()
    events = world.poll_ecs_event("vehicle_disembarked")
    assert len(events) == 1, "one vehicle_disembarked event expected"

    ok, err = world.disembark_vehicle(rider)
    assert ok is False, "second disembark should fail"
    assert err == "not_mounted", "err should be not_mounted"


def test_comove_speed_one_with_two_riders(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider_a = _spawn_rider(world, 0, 0)
    rider_b = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider_a)
    world.embark_vehicle(vehicle, rider_b)
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [rider_a, rider_b],
            "move_path": [_square_cell(1, 0), _square_cell(2, 0), _square_cell(3, 0)],
        },
    )

    for tick, x in enumerate((1, 2, 3), start=1):
        world.tick()
        vpos = _pos_of(world, vehicle)
        assert vpos["Square"]["x"] == x, f"vehicle x after tick {tick}"
        for rider in (rider_a, rider_b):
            rpos = _pos_of(world, rider)
            assert rpos["Square"]["x"] == x, f"rider x must track vehicle after tick {tick}"
            assert rpos["Square"]["y"] == 0, f"rider y must track vehicle after tick {tick}"
        in_cell = world.entities_in_cell(_square_cell(x, 0))
        for eid in (vehicle, rider_a, rider_b):
            assert eid in in_cell, f"entity {eid} must be in the vehicle cell"


def test_comove_speed_two(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0, speed=2)
    rider = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider)
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 2,
            "blocked_terrains": [],
            "occupants": [rider],
            "move_path": [_square_cell(1, 0), _square_cell(2, 0)],
        },
    )

    world.tick()

    vpos = _pos_of(world, vehicle)
    assert vpos["Square"]["x"] == 2, "speed 2 must advance two cells per tick"
    rpos = _pos_of(world, rider)
    assert rpos["Square"]["x"] == 2, "rider must track the speed-2 vehicle"


def test_hex_comove_preserves_variant(make_world):
    world = make_world()
    world.apply_generated_map(
        {
            "topology": "hex",
            "cells": [{"q": 0, "r": 0, "z": 0}, {"q": 1, "r": 0, "z": 0}],
        }
    )
    world.add_neighbor((0, 0, 0), (1, 0, 0))
    world.add_neighbor((1, 0, 0), (0, 0, 0))

    vehicle = world.spawn_entity()
    world.set_component(vehicle, "Position", {"pos": {"Hex": {"q": 0, "r": 0, "z": 0}}})
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [],
            "move_path": [{"Hex": {"q": 1, "r": 0, "z": 0}}],
        },
    )
    rider = world.spawn_entity()
    world.set_component(rider, "Position", {"pos": {"Hex": {"q": 0, "r": 0, "z": 0}}})
    world.embark_vehicle(vehicle, rider)

    world.tick()

    vpos = _pos_of(world, vehicle)
    assert vpos["Hex"]["q"] == 1, "hex vehicle must advance one step"
    rpos = _pos_of(world, rider)
    assert rpos["Hex"]["q"] == 1, "hex rider must track the vehicle"
    assert "Hex" in rpos, "rider Position must stay Hex-variant"


def test_capacity_reject_emits_rejection(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0, capacity=1)
    rider_a = _spawn_rider(world, 0, 0)
    rider_b = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider_a)

    ok, err = world.embark_vehicle(vehicle, rider_b)
    assert ok is False, "second embark on capacity-1 vehicle should fail"
    assert err == "full", "err should be full"
    assert world.get_vehicle_occupants(vehicle) == [rider_a], "occupants must be unchanged"
    assert world.is_mounted(rider_b) is False, "rejected rider must not be mounted"

    world.update_event_buses()
    events = world.poll_ecs_event("vehicle_embark_rejected")
    assert len(events) == 1, "one rejection event expected"
    assert events[0]["reason"] == "full", "rejection reason must be full"


def test_terrain_guards_truncate_and_emit_blocked(make_world):
    world = make_world()
    _make_corridor(world)
    vehicle = _spawn_vehicle(world, 0, 0, blocked=["water"])
    rider = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider)
    world.set_cell_metadata(_square_cell(1, 0), {"walkable": False})
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": ["water"],
            "occupants": [rider],
            "move_path": [_square_cell(1, 0), _square_cell(2, 0)],
        },
    )

    world.tick()

    vpos = _pos_of(world, vehicle)
    assert vpos["Square"]["x"] == 0, "vehicle must hold before the unwalkable step"
    rpos = _pos_of(world, rider)
    assert rpos["Square"]["x"] == 0, "rider must hold with the vehicle"
    blocked = world.poll_ecs_event("vehicle_move_blocked")
    assert len(blocked) == 1, "one vehicle_move_blocked event expected"


def test_assign_path_prefix_and_errors(make_world):
    world = make_world()
    _make_corridor(world)
    vehicle = _spawn_vehicle(world, 0, 0, blocked=["water"])
    world.set_cell_metadata(_square_cell(2, 0), {"terrain": "water"})

    steps = world.assign_vehicle_path(vehicle, _square_cell(3, 0))
    assert steps == 1, "stored path must be the pre-blockage prefix"
    stored = world.get_component(vehicle, "Vehicle")
    assert len(stored["move_path"]) == 1, "one prefix step must be stored"

    world.set_cell_metadata(_square_cell(1, 0), {"terrain": "water"})
    steps_empty = world.assign_vehicle_path(vehicle, _square_cell(3, 0))
    assert steps_empty == 0, "fully-blocked goal must store an empty path"

    with pytest.raises(ValueError, match="no_vehicle"):
        world.assign_vehicle_path(9999, _square_cell(1, 0))


def test_mounted_rider_path_suppressed(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider)
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [rider],
            "move_path": [_square_cell(1, 0), _square_cell(2, 0), _square_cell(3, 0)],
        },
    )

    for tick in range(1, 4):
        world.set_component(
            rider,
            "Agent",
            {"entity_id": rider, "move_path": [_square_cell(0, 5)]},
        )
        world.tick()
        vpos = _pos_of(world, vehicle)
        rpos = _pos_of(world, rider)
        assert rpos["Square"]["x"] == vpos["Square"]["x"], (
            f"rider must track vehicle on tick {tick}"
        )
        assert rpos["Square"]["y"] == vpos["Square"]["y"], (
            f"rider must track vehicle on tick {tick}"
        )


def test_despawn_leaves_rider_alive_at_last_cell(make_world):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider)
    vpos = _pos_of(world, vehicle)

    world.despawn_entity(vehicle)
    world.tick()

    assert rider in world.get_entities(), "rider must survive the vehicle"
    assert world.is_mounted(rider) is False, "rider must not be mounted after despawn"
    rpos = _pos_of(world, rider)
    assert rpos["Square"]["x"] == vpos["Square"]["x"], "rider must keep the last vehicle cell"


def test_province_vehicle_holds_with_riders_synced(make_world):
    world = make_world()
    world.register_map(
        "overmap",
        {"topology": "province", "cells": [{"id": "prov_a"}, {"id": "prov_b"}]},
    )
    world.set_active_map("overmap")

    vehicle = world.spawn_entity()
    world.set_component(vehicle, "Position", {"pos": {"Province": {"id": "prov_a"}}})
    world.set_component(
        vehicle,
        "Vehicle",
        {
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [],
            "move_path": [{"Province": {"id": "prov_b"}}],
        },
    )
    rider = world.spawn_entity()
    world.set_component(rider, "Position", {"pos": {"Province": {"id": "prov_a"}}})
    world.embark_vehicle(vehicle, rider)

    world.tick()

    vpos = _pos_of(world, vehicle)
    assert vpos["Province"]["id"] == "prov_a", "province vehicle must hold position"
    rpos = _pos_of(world, rider)
    assert rpos["Province"]["id"] == "prov_a", "province rider must stay synced"


def test_save_load_roundtrip_preserves_mounted_vehicle(make_world, tmp_path):
    world = make_world()
    _make_open_plane(world)
    vehicle = _spawn_vehicle(world, 0, 0)
    rider_a = _spawn_rider(world, 0, 0)
    rider_b = _spawn_rider(world, 0, 0)
    world.embark_vehicle(vehicle, rider_a)
    world.embark_vehicle(vehicle, rider_b)
    world.assign_vehicle_path(vehicle, _square_cell(4, 0))
    world.tick()
    world.tick()

    save_file = tmp_path / "test_vehicle_save.json"
    world.save_to_file(str(save_file))
    for eid in world.get_entities():
        world.despawn_entity(eid)
    world.load_from_file(str(save_file))

    occupants = world.get_vehicle_occupants(vehicle)
    assert occupants == [rider_a, rider_b], "two riders must survive round-trip"
    assert world.is_mounted(rider_a) is True, "rider_a must stay mounted"
    assert world.is_mounted(rider_b) is True, "rider_b must stay mounted"
    vpos = _pos_of(world, vehicle)
    for rider in (rider_a, rider_b):
        rpos = _pos_of(world, rider)
        assert rpos["Square"]["x"] == vpos["Square"]["x"], "rider must stay co-located"
        assert rpos["Square"]["y"] == vpos["Square"]["y"], "rider must stay co-located"
