"""Parity tests for the Crafting system (ROADMAP L57) via the Python API."""

import json


RECIPE = "iron_sword"


def _sword_recipe():
    return json.dumps(
        {
            "name": RECIPE,
            "inputs": [{"kind": "iron", "amount": 2}],
            "outputs": [],
            "duration": 3,
            "tools": [{"item": "hammer", "consumed": False}],
            "materials": [{"material": "iron", "amount": 2}],
            "required_skill": {"skill": "crafting", "level": 2},
            "output_item": {"id": "iron_sword", "name": "Iron Sword", "slot": "hand"},
            "station": None,
            "xp": 12,
        }
    )


def _spawn_crafter(world, iron=200):
    eid = world.spawn_entity()
    world.set_component(eid, "Stockpile", {"resources": {"iron": iron}})
    world.set_component(
        eid,
        "Inventory",
        {"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0},
    )
    world.set_component(
        eid,
        "SkillLevels",
        {
            "skills": {"crafting": 2.0},
            "total_xp": 0.0,
            "skill_xp": {},
            "skill_levels": {},
        },
    )
    return eid


def _setup(world):
    world.register_craft_recipe(RECIPE, _sword_recipe())
    return _spawn_crafter(world)


def _stockpile_iron(world, eid):
    return world.get_component(eid, "Stockpile")["resources"]["iron"]


def _complete(world, crafter):
    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is True, f"start should succeed, got {err}"
    for _ in range(3):
        world.tick()


def test_tool_gate_names_missing_hammer(make_world):
    world = make_world()
    crafter = _setup(world)
    world.set_component(
        crafter,
        "Inventory",
        {"slots": [], "max_slots": 10, "weight": 0.0, "volume": 0.0},
    )

    ok, err = world.can_craft(crafter, RECIPE)
    assert ok is False, "gate without hammer should fail"
    assert err == "missing_tool:hammer", "gate must name the missing tool"
    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is False, "start without hammer should fail"
    assert err == "missing_tool:hammer", "start must name the missing tool"
    assert world.get_craft_state(crafter) is None, "failed start must not create an order"

    world.set_component(
        crafter,
        "Inventory",
        {"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0},
    )
    ok, err = world.can_craft(crafter, RECIPE)
    assert ok is True, "gate with hammer should pass"
    assert err is None, "err should be None on success"


def test_consumed_tool_removed_once(make_world):
    world = make_world()
    world.register_craft_recipe(
        RECIPE,
        json.dumps(
            {
                "name": RECIPE,
                "inputs": [],
                "outputs": [],
                "duration": 3,
                "tools": [{"item": "hammer", "consumed": True}],
                "materials": [],
                "output_item": {"id": "iron_sword", "name": "Iron Sword", "slot": "hand"},
            }
        ),
    )
    crafter = world.spawn_entity()
    world.set_component(
        crafter,
        "Inventory",
        {"slots": ["hammer", "hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0},
    )

    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is True, f"start should succeed, got {err}"
    slots = world.get_component(crafter, "Inventory")["slots"]
    assert slots == ["hammer"], "consumed tool must be removed exactly once"


def test_materials_deducted_and_output_spawned(make_world):
    world = make_world()
    crafter = _setup(world)
    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is True, f"start should succeed, got {err}"
    assert _stockpile_iron(world, crafter) == 196, "inputs plus materials deduct at start"

    world.tick()
    world.tick()
    assert world.get_craft_state(crafter)["progress"] == 2, "two ticks advance to 2"
    world.tick()

    state = world.get_craft_state(crafter)
    assert state["state"] == "complete", "third tick completes a duration-3 recipe"
    assert state["progress"] == 3, "completion stores terminal progress"
    output = state["output_entity"]
    assert output is not None, "completed order must record the output entity"

    item = world.get_component(output, "Item")
    assert item == {
        "id": "iron_sword",
        "name": "Iron Sword",
        "slot": "hand",
        "material": "iron",
    }, "output item payload mismatch"
    material = world.get_component(output, "Material")
    assert material["material"] == "iron", "output material key mismatch"
    assert 0.0 <= material["quality"] <= 10.0, "quality must be clamped to range"
    slots = world.get_component(crafter, "Inventory")["slots"]
    assert "iron_sword" in slots, "roomy inventory must receive the output"


def test_skill_gate_and_xp(make_world):
    world = make_world()
    crafter = _setup(world)
    world.set_component(
        crafter,
        "SkillLevels",
        {
            "skills": {"crafting": 1.0},
            "total_xp": 0.0,
            "skill_xp": {},
            "skill_levels": {},
        },
    )
    ok, err = world.can_craft(crafter, RECIPE)
    assert ok is False, "below-gate skill should fail"
    assert err == "insufficient_skill", "skill shortfall must report insufficient_skill"

    world.set_component(
        crafter,
        "SkillLevels",
        {
            "skills": {"crafting": 2.0},
            "total_xp": 0.0,
            "skill_xp": {},
            "skill_levels": {},
        },
    )
    _complete(world, crafter)

    world.update_event_buses()
    events = world.poll_ecs_event("craft_completed")
    assert len(events) == 1, "one craft_completed event expected"
    payload = events[0]
    assert payload["entity"] == crafter, "event entity mismatch"
    assert payload["recipe"] == RECIPE, "event recipe mismatch"
    assert payload["output_entity"] is not None, "event must carry output_entity"
    assert payload["material"] == "iron", "event material mismatch"
    assert payload["xp_gained"] in (11, 12), "xp override 12 plus jitter floors to 11 or 12"
    levels = world.get_component(crafter, "SkillLevels")
    assert levels["total_xp"] == payload["xp_gained"], "xp grant must match the event"
    assert levels["skill_xp"]["crafting"] == payload["xp_gained"], "per-skill xp must match"


def test_error_parity_strings(make_world):
    world = make_world()
    crafter = _setup(world)

    ok, err = world.can_craft(crafter, "ghost_recipe")
    assert ok is False, "unknown recipe should fail"
    assert err == "unknown_recipe", "unknown recipe string mismatch"

    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is True, f"first start should succeed, got {err}"
    ok, err = world.start_craft(crafter, RECIPE)
    assert ok is False, "double start should fail"
    assert err == "already_crafting", "double start string mismatch"

    other = world.spawn_entity()
    ok, err = world.cancel_craft(other)
    assert ok is False, "cancel without order should fail"
    assert err == "no_craft_order", "missing order string mismatch"


def test_cancel_refunds_and_emits(make_world):
    world = make_world()
    crafter = _setup(world)
    world.start_craft(crafter, RECIPE)
    assert _stockpile_iron(world, crafter) == 196, "start must deduct before cancel"

    ok, err = world.cancel_craft(crafter)
    assert ok is True, f"cancel should succeed, got {err}"
    assert err is None, "err should be None on cancel"
    assert _stockpile_iron(world, crafter) == 200, "cancel must refund stockpile inputs"
    assert world.get_craft_state(crafter) is None, "cancel must remove the order"

    world.update_event_buses()
    events = world.poll_ecs_event("craft_cancelled")
    assert len(events) == 1, "one craft_cancelled event expected"
    assert events[0]["entity"] == crafter, "cancel event entity mismatch"
    assert events[0]["refunded"] is True, "cancel event must report refunded"


def test_station_ignored_and_stockpile_only_unknown(make_world):
    world = make_world()
    world.register_craft_recipe(
        RECIPE,
        json.dumps(
            {
                "name": RECIPE,
                "inputs": [],
                "outputs": [],
                "duration": 1,
                "tools": [],
                "materials": [],
                "station": "bogus_workbench",
                "output_item": {"id": "iron_sword", "name": "Iron Sword", "slot": "hand"},
            }
        ),
    )
    crafter = world.spawn_entity()
    ok, err = world.can_craft(crafter, RECIPE)
    assert ok is True, f"bogus station must not gate, got {err}"

    world.register_craft_recipe(
        "plank_batch",
        json.dumps(
            {
                "name": "plank_batch",
                "inputs": [{"kind": "wood", "amount": 1}],
                "outputs": [{"kind": "plank", "amount": 2}],
                "duration": 2,
            }
        ),
    )
    ok, err = world.can_craft(crafter, "plank_batch")
    assert ok is False, "stockpile-only recipe should fail on the craft path"
    assert err == "unknown_recipe", "stockpile-only recipes stay unknown to craft"


def test_full_inventory_leaves_world_entity(make_world):
    world = make_world()
    crafter = _setup(world)
    world.set_component(
        crafter,
        "Inventory",
        {"slots": ["hammer"], "max_slots": 1, "weight": 0.0, "volume": 0.0},
    )
    _complete(world, crafter)

    world.update_event_buses()
    events = world.poll_ecs_event("craft_completed")
    assert len(events) == 1, "completion must still fire when inventory is full"
    output = events[0]["output_entity"]
    assert output is not None, "event must carry the world-entity fallback"
    assert world.get_component(output, "Item")["id"] == "iron_sword"
    slots = world.get_component(crafter, "Inventory")["slots"]
    assert slots == ["hammer"], "full inventory must not gain the output"


def _snapshot(world):
    ids = sorted(world.get_entities())
    parts = []
    for eid in ids:
        entry = {"entity": eid}
        for name in ("Stockpile", "CraftOrder", "Item", "Material", "SkillLevels", "Inventory"):
            try:
                entry[name] = world.get_component(eid, name)
            except Exception:
                pass
        parts.append(entry)
    world.update_event_buses()
    events = [json.dumps(e, sort_keys=True) for e in world.poll_ecs_event("craft_completed")]
    return json.dumps({"turn": world.get_turn(), "entities": parts, "events": events}, sort_keys=True)


def test_fifty_tick_two_world_determinism(make_world):
    world_a = make_world()
    crafter_a = _setup(world_a)
    world_b = make_world()
    crafter_b = _setup(world_b)
    assert crafter_a == crafter_b, "identical setups must align entity ids"

    for tick in range(50):
        for world, crafter in ((world_a, crafter_a), (world_b, crafter_b)):
            state = world.get_craft_state(crafter)
            if state is None or state.get("state") != "in_progress":
                ok, err = world.start_craft(crafter, RECIPE)
                assert ok is True, f"restart should succeed on tick {tick}, got {err}"
            world.tick()
        assert _snapshot(world_a) == _snapshot(world_b), f"craft state diverged on tick {tick}"
        assert world_a.get_turn() == world_b.get_turn(), f"turn diverged on tick {tick}"


def test_save_load_roundtrip_preserves_order(make_world, tmp_path):
    world = make_world()
    crafter = _setup(world)
    world.start_craft(crafter, RECIPE)
    world.tick()

    save_file = tmp_path / "test_crafting_save.json"
    world.save_to_file(str(save_file))
    for eid in world.get_entities():
        world.despawn_entity(eid)
    world.load_from_file(str(save_file))

    assert world.list_craft_recipes() == [RECIPE], "recipes must survive round-trip"
    state = world.get_craft_state(crafter)
    assert state is not None, "in-progress order must survive round-trip"
    assert state["recipe"] == RECIPE, "recipe must survive round-trip"
    assert state["progress"] == 1, "progress must survive round-trip"
    assert state["state"] == "in_progress", "order state must survive round-trip"
    assert _stockpile_iron(world, crafter) == 196, "deductions must survive round-trip"

    world.tick()
    world.tick()
    done = world.get_craft_state(crafter)
    assert done["state"] == "complete", "loaded world must tick to completion"
    assert done["output_entity"] is not None, "loaded completion must record the output"
    assert world.get_component(done["output_entity"], "Item")["id"] == "iron_sword"
