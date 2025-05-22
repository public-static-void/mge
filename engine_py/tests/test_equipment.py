import pytest


def test_equip_and_unequip(make_world):
    w = make_world()
    e = w.spawn_entity()
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    sword = w.spawn_entity()
    w.set_component(
        sword, "Item", {"id": "sword", "name": "Sword", "slot": "right_hand"}
    )
    w.add_item_to_inventory(e, "sword")

    # Equip the sword
    w.equip_item(e, "sword", "right_hand")
    eq = w.get_equipment(e)
    assert eq["slots"]["right_hand"] == "sword"

    # Unequip the sword
    w.unequip_item(e, "right_hand")
    eq = w.get_equipment(e)
    assert eq["slots"]["right_hand"] is None


def test_equip_invalid_slot(make_world):
    w = make_world()
    e = w.spawn_entity()
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    sword = w.spawn_entity()
    w.set_component(
        sword, "Item", {"id": "sword", "name": "Sword", "slot": "right_hand"}
    )
    w.add_item_to_inventory(e, "sword")
    with pytest.raises(Exception) as excinfo:
        w.equip_item(e, "sword", "left_foot")
    assert "invalid slot" in str(excinfo.value)


def test_equip_item_not_in_inventory(make_world):
    w = make_world()
    e = w.spawn_entity()
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    sword = w.spawn_entity()
    w.set_component(
        sword, "Item", {"id": "sword", "name": "Sword", "slot": "right_hand"}
    )
    with pytest.raises(Exception) as excinfo:
        w.equip_item(e, "sword", "right_hand")
    assert "not in inventory" in str(excinfo.value)


def test_double_equip_same_slot(make_world):
    w = make_world()
    e = w.spawn_entity()
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    sword = w.spawn_entity()
    w.set_component(
        sword, "Item", {"id": "sword", "name": "Sword", "slot": "right_hand"}
    )
    shield = w.spawn_entity()
    w.set_component(
        shield,
        "Item",
        {"id": "shield", "name": "Shield", "slot": "right_hand"},
    )
    w.add_item_to_inventory(e, "sword")
    w.add_item_to_inventory(e, "shield")
    w.equip_item(e, "sword", "right_hand")
    with pytest.raises(Exception) as excinfo:
        w.equip_item(e, "shield", "right_hand")
    assert "already equipped" in str(excinfo.value)
