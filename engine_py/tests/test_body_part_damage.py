def humanoid_body():
    return {
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "hp": 50.0,
                "max_hp": 50.0,
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "hp": 25.0,
                        "max_hp": 25.0,
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
                                "hp": 10.0,
                                "max_hp": 10.0,
                                "temperature": 34.0,
                                "ideal_temperature": 36.5,
                                "insulation": 0.5,
                                "heat_loss": 0.3,
                                "children": [],
                                "equipped": [],
                            }
                        ],
                        "equipped": [],
                    }
                ],
                "equipped": [],
            }
        ],
    }


# AC015: damage_entity with Body distributes to parts
def test_damage_entity_distributes(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_body(e, humanoid_body())
    world.set_component(e, "Health", {"current": 85.0, "max": 85.0})

    world.damage_entity(e, 85.0)
    world.run_native_system("BodyPartDamageSystem")

    body = world.get_component(e, "Body")
    torso = body["parts"][0]
    assert torso["status"] == "broken"
    assert torso["hp"] == 0.0


# AC016: damage_entity_part targets specific part
def test_damage_entity_part_targets(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_body(e, humanoid_body())
    world.set_component(e, "Health", {"current": 85.0, "max": 85.0})

    world.damage_entity_part(e, "left hand", 5.0)
    world.run_native_system("BodyPartDamageSystem")

    body = world.get_component(e, "Body")
    hand = body["parts"][0]["children"][0]["children"][0]
    assert hand["hp"] == 5.0
    assert hand["status"] == "wounded"
