import pytest


def test_inventory_crud(make_world):
    world = make_world()
    e = world.spawn_entity()
    # Set inventory with required fields
    inv = {"slots": [], "max_slots": 5, "weight": 0.0, "volume": 0.0}
    world.set_inventory(e, inv)
    got = world.get_inventory(e)
    assert got["max_slots"] == 5
    assert got["slots"] == []
    assert got["weight"] == 0.0
    assert got["volume"] == 0.0


def test_add_and_remove_item(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_inventory(e, {"slots": [], "weight": 0.0, "volume": 0.0})
    item_id = "sword"
    item_entity = world.spawn_entity()
    world.set_component(
        item_entity,
        "Item",
        {"id": item_id, "name": "Sword", "slot": "right_hand"},
    )
    world.add_item_to_inventory(e, item_id)
    inv = world.get_inventory(e)
    assert inv["slots"][0] == item_id
    # Remove the item
    world.remove_item_from_inventory(e, 0)
    inv = world.get_inventory(e)
    assert inv["slots"] == []


def test_remove_item_out_of_bounds(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_inventory(e, {"slots": [], "weight": 0.0, "volume": 0.0})
    with pytest.raises(Exception):
        world.remove_item_from_inventory(e, 0)
