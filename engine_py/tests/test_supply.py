import math

import pytest


def make_stocked(world, kind="grain", a_amount=100.0, b_amount=0.0):
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


def test_create_link_returns_record_verbatim(make_world):
    world = make_world()
    a, b = make_stocked(world)
    link = world.create_supply_link(a, b, "grain", 10.0, 25.0)
    record = world.get_supply_link(link)
    assert record["source"] == a
    assert record["target"] == b
    assert record["kind"] == "grain"
    assert record["amount_per_tick"] == 10.0
    assert record["capacity_per_tick"] == 25.0
    assert record["active"] is True


def test_create_link_rejects_bad_input(make_world):
    world = make_world()
    a, b = make_stocked(world)
    bare = world.spawn_entity()

    with pytest.raises(ValueError, match="SameEndpoint"):
        world.create_supply_link(a, a, "grain", 10.0, 25.0)
    with pytest.raises(ValueError, match="UnknownKind"):
        world.create_supply_link(a, b, "", 10.0, 25.0)
    for amount in (0.0, -1.0, math.nan, math.inf):
        with pytest.raises(ValueError, match="NonPositiveAmount"):
            world.create_supply_link(a, b, "grain", amount, 25.0)
        with pytest.raises(ValueError, match="NonPositiveAmount"):
            world.create_supply_link(a, b, "grain", 10.0, amount)
    with pytest.raises(ValueError, match="NoStockpile"):
        world.create_supply_link(bare, b, "grain", 10.0, 25.0)
    with pytest.raises(ValueError, match="NoStockpile"):
        world.create_supply_link(a, bare, "grain", 10.0, 25.0)

    assert world.list_supply_links() == []


def test_unknown_link_errors(make_world):
    world = make_world()
    bogus = 999999
    with pytest.raises(ValueError, match="UnknownLink"):
        world.remove_supply_link(bogus)
    with pytest.raises(ValueError, match="UnknownLink"):
        world.set_supply_link_active(bogus, False)
    with pytest.raises(ValueError, match="UnknownLink"):
        world.get_supply_link(bogus)


def test_list_returns_links_in_ascending_order(make_world):
    world = make_world()
    a, b = make_stocked(world)
    c = world.spawn_entity()
    world.set_component(c, "Stockpile", {"resources": {"grain": 50.0}})
    first = world.create_supply_link(a, b, "grain", 10.0, 25.0)
    second = world.create_supply_link(b, c, "grain", 5.0, 5.0)
    assert world.list_supply_links() == sorted([first, second])


def test_remove_link_drops_it_from_list_and_query(make_world):
    world = make_world()
    a, b = make_stocked(world)
    link = world.create_supply_link(a, b, "grain", 10.0, 25.0)
    assert world.list_supply_links() == [link]
    world.remove_supply_link(link)
    assert world.list_supply_links() == []
    with pytest.raises(ValueError, match="UnknownLink"):
        world.get_supply_link(link)


def test_tick_delivers_capped_request(make_world):
    world = make_world()
    a, b = make_stocked(world)
    world.create_supply_link(a, b, "grain", 10.0, 25.0)
    world.tick()
    assert balance(world, a) == 90.0
    assert balance(world, b) == 10.0
    events = world.poll_ecs_event("supply_delivered")
    assert len(events) == 1
    assert events[0]["kind"] == "grain"
    assert events[0]["amount"] == 10.0


def test_tick_respects_capacity_cap(make_world):
    world = make_world()
    a, b = make_stocked(world)
    world.create_supply_link(a, b, "grain", 30.0, 12.0)
    world.tick()
    assert balance(world, a) == 88.0
    assert balance(world, b) == 12.0


def test_inactive_link_skips_silently_then_resumes(make_world):
    world = make_world()
    a, b = make_stocked(world)
    link = world.create_supply_link(a, b, "grain", 10.0, 25.0)
    world.set_supply_link_active(link, False)
    assert world.get_supply_link(link)["active"] is False
    world.tick()
    assert balance(world, a) == 100.0
    assert balance(world, b) == 0.0
    assert world.poll_ecs_event("supply_delivered") == []
    assert world.poll_ecs_event("supply_shortfall") == []
    assert world.poll_ecs_event("supply_blocked") == []
    world.set_supply_link_active(link, True)
    assert world.get_supply_link(link)["active"] is True
    world.tick()
    assert balance(world, a) == 90.0
    assert balance(world, b) == 10.0
