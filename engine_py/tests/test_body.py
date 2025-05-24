import pytest


def empty_part(name):
    return {
        "name": name,
        "status": "healthy",
        "kind": "flesh",
        "temperature": 37.0,
        "ideal_temperature": 37.0,
        "insulation": 1.0,
        "heat_loss": 0.1,
        "children": [],
        "equipped": [],
    }


def test_body_get_set(make_world):
    world = make_world()
    e = world.spawn_entity()
    body = {"parts": [empty_part("torso")]}
    world.set_body(e, body)
    got = world.get_body(e)
    assert isinstance(got, dict)
    assert got["parts"][0]["name"] == "torso"


def test_body_add_remove_part(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_body(e, {"parts": []})

    # Add torso
    world.add_body_part(e, empty_part("torso"))
    body = world.get_body(e)
    assert len(body["parts"]) == 1
    assert body["parts"][0]["name"] == "torso"

    # Add left_arm as child of torso
    torso = body["parts"][0]
    if "children" not in torso or torso["children"] is None:
        torso["children"] = []
    torso["children"].append(empty_part("left_arm"))
    world.set_body(e, body)
    body = world.get_body(e)
    assert len(body["parts"][0]["children"]) == 1
    assert body["parts"][0]["children"][0]["name"] == "left_arm"

    # Remove left_arm
    world.remove_body_part(e, "left_arm")
    body = world.get_body(e)
    assert len(body["parts"][0]["children"]) == 0


def test_body_get_body_part(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_body(e, {"parts": []})
    world.add_body_part(e, empty_part("torso"))
    torso = world.get_body_part(e, "torso")
    assert isinstance(torso, dict)
    assert torso["name"] == "torso"
    assert torso["status"] == "healthy"
