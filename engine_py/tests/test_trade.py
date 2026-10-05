import math

import pytest


def make_stocked(world, kind="grain", a_amount=10.0, b_amount=0.0):
    a = world.spawn_entity()
    b = world.spawn_entity()
    world.set_component(a, "Stockpile", {"resources": {kind: a_amount}})
    world.set_component(b, "Stockpile", {"resources": {kind: b_amount}})
    return a, b


def balance(world, entity, kind="grain"):
    stockpile = world.get_component(entity, "Stockpile")
    if stockpile is None:
        return None
    return stockpile["resources"].get(kind)


def test_transfer_moves_balance(make_world):
    world = make_world()
    a, b = make_stocked(world)
    world.transfer_stockpile_resource(a, b, "grain", 4.0)
    assert balance(world, a) == 6.0
    assert balance(world, b) == 4.0


def test_transfer_insufficient_leaves_unchanged(make_world):
    world = make_world()
    a, b = make_stocked(world)
    with pytest.raises(ValueError, match="InsufficientFunds"):
        world.transfer_stockpile_resource(a, b, "grain", 99.0)
    assert balance(world, a) == 10.0
    assert balance(world, b) == 0.0


def test_transfer_rejects_bad_kind_and_amount(make_world):
    world = make_world()
    a, b = make_stocked(world)
    with pytest.raises(ValueError, match="UnknownKind"):
        world.transfer_stockpile_resource(a, b, "", 1.0)
    for amount in (0.0, -1.0, math.nan, math.inf):
        with pytest.raises(ValueError, match="NonPositiveAmount"):
            world.transfer_stockpile_resource(a, b, "grain", amount)
    assert balance(world, a) == 10.0
    assert balance(world, b) == 0.0


def test_transfer_missing_stockpile_errors(make_world):
    world = make_world()
    a, b = make_stocked(world)
    bare = world.spawn_entity()
    with pytest.raises(ValueError, match="NoStockpile"):
        world.transfer_stockpile_resource(bare, b, "grain", 1.0)
    with pytest.raises(ValueError, match="NoStockpile"):
        world.transfer_stockpile_resource(a, bare, "grain", 1.0)
    assert balance(world, a) == 10.0
    assert balance(world, b) == 0.0


def test_transfer_self_is_validated_noop(make_world):
    world = make_world()
    a, _ = make_stocked(world)
    world.transfer_stockpile_resource(a, a, "grain", 2.0)
    assert balance(world, a) == 10.0


def make_treaty_pair(world):
    a, b = make_stocked(world)
    world.set_faction(a, "f1", "member")
    world.set_faction(b, "f2", "member")
    return a, b


def test_treaty_lifecycle_gates_execution(make_world):
    world = make_world()
    a, b = make_treaty_pair(world)
    assert world.has_active_trade_treaty("f1", "f2") is False

    tid = world.propose_treaty("f1", "f2", "trade")
    world.accept_treaty(tid)
    assert world.has_active_trade_treaty("f1", "f2") is True
    assert world.has_active_trade_treaty("f2", "f1") is True

    world.execute_treaty_trade(a, b, "grain", 3.0)
    assert balance(world, a) == 7.0
    assert balance(world, b) == 3.0

    world.declare_war("f1", "f2")
    assert world.has_active_trade_treaty("f1", "f2") is True
    with pytest.raises(ValueError, match="RelationIsWar"):
        world.execute_treaty_trade(a, b, "grain", 1.0)
    assert balance(world, a) == 7.0
    assert balance(world, b) == 3.0

    world.declare_peace("f1", "f2")
    world.execute_treaty_trade(a, b, "grain", 1.0)
    assert balance(world, a) == 6.0
    assert balance(world, b) == 4.0


def test_gated_trade_without_treaty_fails(make_world):
    world = make_world()
    a, b = make_treaty_pair(world)
    with pytest.raises(ValueError, match="NoLiveTreaty"):
        world.execute_treaty_trade(a, b, "grain", 1.0)
    assert balance(world, a) == 10.0
    assert balance(world, b) == 0.0


def test_gated_trade_without_faction_fails(make_world):
    world = make_world()
    a, b = make_stocked(world)
    with pytest.raises(ValueError, match="NoLiveTreaty"):
        world.execute_treaty_trade(a, b, "grain", 1.0)
    assert balance(world, a) == 10.0
    assert balance(world, b) == 0.0


def make_upkeep_entity(world, stock, drains):
    e = world.spawn_entity()
    world.set_component(e, "Stockpile", {"resources": stock})
    world.set_component(e, "Upkeep", {"drains": drains})
    return e


def test_upkeep_roundtrip_and_rejection(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_component(e, "Upkeep", {"drains": [{"kind": "grain", "amount_per_tick": 2.0}]})
    upkeep = world.get_component(e, "Upkeep")
    assert upkeep["drains"][0]["kind"] == "grain"
    assert upkeep["drains"][0]["amount_per_tick"] == 2.0

    bad_cases = [
        {"drains": [{"kind": "", "amount_per_tick": 2.0}]},
        {"drains": [{"kind": "grain", "amount_per_tick": 0.0}]},
        {"drains": [{"kind": "grain", "amount_per_tick": -1.0}]},
        {"drains": [{"kind": "grain", "amount_per_tick": math.nan}]},
        {"drains": [{"kind": "grain", "amount_per_tick": math.inf}]},
    ]
    for bad in bad_cases:
        with pytest.raises(ValueError):
            world.set_component(e, "Upkeep", bad)


def test_upkeep_drains_each_tick(make_world):
    world = make_world()
    e = make_upkeep_entity(world, {"grain": 10.0}, [{"kind": "grain", "amount_per_tick": 2.0}])
    world.tick()
    assert balance(world, e) == 8.0
    world.tick()
    world.tick()
    assert balance(world, e) == 4.0


def test_upkeep_shortage_keeps_balance_and_emits_event(make_world):
    world = make_world()
    e = make_upkeep_entity(world, {"grain": 1.0}, [{"kind": "grain", "amount_per_tick": 2.0}])
    world.tick()
    assert balance(world, e) == 1.0
    events = world.poll_ecs_event("consumption_shortage")
    assert len(events) == 1
    assert events[0]["entity"] == e
    assert events[0]["kind"] == "grain"
    assert events[0]["required"] == 2.0
    assert events[0]["available"] == 1.0


def test_upkeep_multi_drain_is_atomic(make_world):
    world = make_world()
    e = make_upkeep_entity(
        world,
        {"grain": 10.0, "wood": 1.0},
        [{"kind": "grain", "amount_per_tick": 2.0}, {"kind": "wood", "amount_per_tick": 2.0}],
    )
    world.tick()
    assert balance(world, e, "grain") == 10.0
    assert balance(world, e, "wood") == 1.0
    events = world.poll_ecs_event("consumption_shortage")
    assert len(events) == 1
    assert events[0]["kind"] == "wood"


def test_upkeep_absent_entity_is_spared(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_component(e, "Stockpile", {"resources": {"grain": 10.0}})
    world.tick()
    assert balance(world, e) == 10.0
