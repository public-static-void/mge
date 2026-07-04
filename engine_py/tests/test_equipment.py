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


def test_equip_stat_effect_bonus(make_world):
    """Equip strength+3 item → Stats.strength = base+3."""
    w = make_world()
    e = w.spawn_entity()
    w.set_component(e, "BaseStats", {"strength": 5.0, "constitution": 3.0})
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    item = w.spawn_entity()
    w.set_component(
        item,
        "Item",
        {"id": "str_ring", "name": "Ring of Strength", "slot": "ring_1", "effects": {"strength": 3.0}},
    )
    w.add_item_to_inventory(e, "str_ring")
    w.equip_item(e, "str_ring", "ring_1")

    # Run the stat pipeline systems
    w.run_native_system("EquipmentEffectAggregationSystem")
    w.run_native_system("StatCalculationSystem")

    stats = w.get_component(e, "Stats")
    assert stats["strength"] == 8.0, f"Expected 8.0, got {stats['strength']}"
    assert stats["constitution"] == 3.0, "Constitution should be unchanged"


def test_unequip_removes_stat_bonus(make_world):
    """Unequip → Stats returns to base values."""
    w = make_world()
    e = w.spawn_entity()
    w.set_component(e, "BaseStats", {"strength": 5.0})
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    item = w.spawn_entity()
    w.set_component(
        item,
        "Item",
        {"id": "str_ring", "name": "Ring of Strength", "slot": "ring_1", "effects": {"strength": 3.0}},
    )
    w.add_item_to_inventory(e, "str_ring")
    w.equip_item(e, "str_ring", "ring_1")

    # Run pipeline: effect aggregated
    w.run_native_system("EquipmentEffectAggregationSystem")
    w.run_native_system("StatCalculationSystem")

    stats = w.get_component(e, "Stats")
    assert stats["strength"] == 8.0, f"Expected 8.0, got {stats['strength']}"

    # Unequip and re-run pipeline
    w.unequip_item(e, "ring_1")
    w.run_native_system("EquipmentEffectAggregationSystem")
    w.run_native_system("StatCalculationSystem")

    stats = w.get_component(e, "Stats")
    assert stats["strength"] == 5.0, f"Expected 5.0, got {stats['strength']}"


def test_equip_custom_stat_effect(make_world):
    """Custom stat (charisma) appears in Stats through equipment effects."""
    w = make_world()
    e = w.spawn_entity()
    w.set_component(e, "BaseStats", {"strength": 5.0})
    w.set_component(
        e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0}
    )
    item = w.spawn_entity()
    w.set_component(
        item,
        "Item",
        {"id": "cha_ring", "name": "Ring of Charisma", "slot": "ring_2", "effects": {"charisma": 2.0}},
    )
    w.add_item_to_inventory(e, "cha_ring")
    w.equip_item(e, "cha_ring", "ring_2")

    w.run_native_system("EquipmentEffectAggregationSystem")
    w.run_native_system("StatCalculationSystem")

    stats = w.get_component(e, "Stats")
    # charisma is only in EquipmentEffects, not BaseStats: should appear in Stats
    assert stats["charisma"] == 2.0, f"Expected charisma=2.0, got {stats.get('charisma')}"
    assert stats["strength"] == 5.0, "Strength should be unchanged"
