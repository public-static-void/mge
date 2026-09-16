import pytest


def setup_build_world(make_world):
    world = make_world()
    world.set_mode("colony")
    world.add_cell(0, 0, 0)
    world.add_cell(1, 0, 0)
    world.add_cell(2, 0, 0)

    agent = world.spawn_entity()
    world.set_component(agent, "Agent", {"entity_id": agent, "state": "idle"})
    world.set_component(agent, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}})
    world.set_component(
        agent,
        "Inventory",
        {
            "max_weight": 100.0,
            "max_slots": 10,
            "max_volume": 100.0,
            "weight": 0.0,
            "slots": [],
            "volume": 0.0,
        },
    )

    stockpile = world.spawn_entity()
    world.set_component(stockpile, "Stockpile", {"resources": {"wood": 2}})
    world.set_component(stockpile, "Position", {"pos": {"Square": {"x": 1, "y": 0, "z": 0}}})

    return world, agent, stockpile


def find_construction_job(world, site):
    for job in world.find_jobs(category="construction"):
        if job.get("target") == site:
            return job["id"]
    return None


def deliver_to_site(world, agent, site):
    job_id = find_construction_job(world, site)
    assert job_id is not None, "Construction job should be posted for the site"
    world.run_resource_reservation_system()
    assert world.get_job(job_id)["state"] == "fetching_resources"
    world.set_job_field(job_id, "assigned_to", agent)
    world.set_job_field(job_id, "delivered_resources", [{"kind": "wood", "amount": 2}])
    world.set_component(agent, "Position", {"pos": {"Square": {"x": 2, "y": 0, "z": 0}}})
    return job_id


def test_place_blueprint_and_query_state(make_world):
    world, agent, stockpile = setup_build_world(make_world)

    site = world.place_blueprint(
        "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 2}], 2
    )
    assert site is not None

    state = world.get_construction_state(site)
    assert state["state"] == "pending"
    assert state["progress"] == 0
    assert state["required_work"] == 2
    assert state["building_type"] == "hut"

    assert world.get_component(site, "ConstructionSite") is not None
    job_id = find_construction_job(world, site)
    assert job_id is not None
    job = world.get_job(job_id)
    assert job["category"] == "construction"
    assert job["job_type"] == "construct"


def test_blueprint_validation_rejects_bad_input(make_world):
    world, agent, stockpile = setup_build_world(make_world)

    with pytest.raises(Exception):
        world.place_blueprint(
            "hut", {"Square": {"x": 9, "y": 9, "z": 0}}, [{"kind": "wood", "amount": 1}], 1
        )

    site = world.place_blueprint(
        "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 1}], 1
    )
    assert site is not None
    with pytest.raises(Exception):
        world.place_blueprint(
            "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 1}], 1
        )
    with pytest.raises(Exception):
        world.place_blueprint("hut", {"Square": {"x": 0, "y": 0, "z": 0}}, [], 1)
    with pytest.raises(Exception):
        world.place_blueprint(
            "hut", {"Square": {"x": 0, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 1}], 0
        )


def test_topology_mismatch_rejected(make_world):
    world = make_world()
    world.set_mode("colony")
    world.register_map("field", {"topology": "hex", "cells": [{"q": 0, "r": 0, "z": 0}]})
    world.set_active_map("field")

    with pytest.raises(Exception):
        world.place_blueprint(
            "hut", {"Square": {"x": 0, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 1}], 1
        )


def test_full_build_completes_with_event(make_world):
    world, agent, stockpile = setup_build_world(make_world)

    site = world.place_blueprint(
        "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 2}], 2
    )
    deliver_to_site(world, agent, site)

    for _ in range(10):
        world.run_native_system("ConstructionSystem")
        if world.get_construction_state(site)["state"] == "complete":
            break

    done = world.get_construction_state(site)
    assert done["state"] == "complete"
    assert done["building_type"] == "hut"

    world.update_event_buses()
    events = world.poll_ecs_event("construction_completed")
    assert any(
        e["site_id"] == site and e["building_id"] == site and e["building_type"] == "hut"
        for e in events
    ), "construction_completed event should carry site, building, and type"

    assert world.get_component(site, "Building") is not None
    assert world.get_component(site, "ConstructionSite") is None
    assert world.get_component(stockpile, "Stockpile")["resources"]["wood"] == 0


def test_cancel_refunds_and_removes_site(make_world):
    world, agent, stockpile = setup_build_world(make_world)

    site = world.place_blueprint(
        "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 2}], 5
    )
    job_id = deliver_to_site(world, agent, site)

    world.run_native_system("ConstructionSystem")
    assert world.get_construction_state(site)["state"] == "in_progress"
    assert world.get_component(stockpile, "Stockpile")["resources"]["wood"] == 0

    assert world.cancel_construction(site) is True
    assert world.get_component(stockpile, "Stockpile")["resources"]["wood"] == 2
    assert world.get_job(job_id)["state"] == "cancelled"
    assert world.get_component(site, "ConstructionSite") is None

    with pytest.raises(Exception):
        world.get_construction_state(site)


def test_demolish_removes_completed_building(make_world):
    world, agent, stockpile = setup_build_world(make_world)

    site = world.place_blueprint(
        "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 2}], 1
    )
    deliver_to_site(world, agent, site)
    for _ in range(10):
        world.run_native_system("ConstructionSystem")
        if world.get_construction_state(site)["state"] == "complete":
            break
    assert world.get_construction_state(site)["state"] == "complete"

    with pytest.raises(Exception):
        world.cancel_construction(site)

    assert world.demolish_building(site) is True
    assert world.get_component(site, "Building") is None
    assert world.get_component(stockpile, "Stockpile")["resources"]["wood"] == 0

    with pytest.raises(Exception):
        world.demolish_building(99999)


def test_mode_gate_outside_colony(make_world):
    world = make_world()
    world.set_mode("roguelike")

    with pytest.raises(Exception):
        world.place_blueprint(
            "hut", {"Square": {"x": 2, "y": 0, "z": 0}}, [{"kind": "wood", "amount": 1}], 1
        )
    with pytest.raises(Exception):
        world.get_construction_state(1)
