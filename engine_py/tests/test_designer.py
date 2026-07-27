import os
import pytest
import tempfile
import json


def _write_json_file(directory, name, data):
    path = os.path.join(directory, name)
    with open(path, "w") as f:
        json.dump(data, f)
    return path


def test_register_and_get_item(make_world):
    w = make_world()
    item_json = json.dumps({"id": "iron_sword", "name": "Iron Sword", "slot": "right_hand"})
    w.register_item(item_json)

    result = w.get_item_definition("iron_sword")
    assert result is not None
    assert result["id"] == "iron_sword"
    assert result["name"] == "Iron Sword"
    assert result["slot"] == "right_hand"


def test_get_item_nonexistent(make_world):
    w = make_world()
    result = w.get_item_definition("nonexistent")
    assert result is None


def test_register_item_missing_id(make_world):
    w = make_world()
    item_json = json.dumps({"name": "Sword", "slot": "right_hand"})
    with pytest.raises(Exception) as excinfo:
        w.register_item(item_json)
    assert "missing required" in str(excinfo.value).lower()


def test_list_item_definitions(make_world):
    w = make_world()
    w.register_item(json.dumps({"id": "sword", "name": "Sword", "slot": "right_hand"}))
    w.register_item(json.dumps({"id": "shield", "name": "Shield", "slot": "shield"}))
    ids = w.list_item_definitions()
    assert "sword" in ids
    assert "shield" in ids


def test_load_item_definitions_from_dir(make_world):
    w = make_world()
    with tempfile.TemporaryDirectory() as tmpdir:
        _write_json_file(tmpdir, "sword.json", {"id": "sword", "name": "Sword", "slot": "right_hand"})
        _write_json_file(tmpdir, "shield.json", {"id": "shield", "name": "Shield", "slot": "shield"})
        w.load_item_definitions(tmpdir)

    result = w.get_item_definition("sword")
    assert result is not None
    assert result["id"] == "sword"


def test_define_and_apply_loadout(make_world):
    w = make_world()
    # Register items
    w.register_item(json.dumps({"id": "iron_sword", "name": "Iron Sword", "slot": "right_hand"}))

    # Define equipment set
    w.define_equipment_set("warrior", {"right_hand": "iron_sword"})

    # Create entity with inventory
    e = w.spawn_entity()
    w.set_component(e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0})

    # Apply loadout
    eid = w.apply_loadout(e, "warrior")
    assert eid == e

    # Verify equipment
    eq = w.get_equipment(e)
    assert eq["slots"]["right_hand"] == "iron_sword"


def test_apply_loadout_unregistered_item(make_world):
    w = make_world()
    # Define set referencing unregistered item
    w.define_equipment_set("bad_set", {"right_hand": "nonexistent_sword"})

    e = w.spawn_entity()
    w.set_component(e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0})

    with pytest.raises(Exception) as excinfo:
        w.apply_loadout(e, "bad_set")
    assert "not found in registry" in str(excinfo.value).lower()


def test_get_loadout_match(make_world):
    w = make_world()
    w.register_item(json.dumps({"id": "sword", "name": "Sword", "slot": "right_hand"}))
    w.define_equipment_set("duelist", {"right_hand": "sword"})

    e = w.spawn_entity()
    w.set_component(e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0})
    w.apply_loadout(e, "duelist")

    result = w.get_loadout(e)
    assert result is not None
    assert result["name"] == "duelist"
    assert result["items"]["right_hand"] == "sword"


def test_get_loadout_no_match(make_world):
    w = make_world()
    w.register_item(json.dumps({"id": "sword", "name": "Sword", "slot": "right_hand"}))
    w.define_equipment_set("duelist", {"right_hand": "sword"})

    e = w.spawn_entity()
    w.set_component(e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0})
    # Don't apply loadout — entity has no equipment
    result = w.get_loadout(e)
    assert result is None


def test_validate_equipment_valid(make_world):
    w = make_world()
    w.register_item(json.dumps({"id": "sword", "name": "Sword", "slot": "right_hand"}))

    e = w.spawn_entity()
    w.set_component(e, "Inventory", {"slots": [], "weight": 0.0, "volume": 0.0})
    w.set_component(e, "Equipment", {"slots": {"right_hand": "sword"}})

    result = w.validate_equipment(e)
    assert result["issues"] == []


def test_validate_equipment_unknown_item(make_world):
    w = make_world()
    # Register item but don't set component correctly
    e = w.spawn_entity()
    w.set_component(e, "Equipment", {"slots": {"right_hand": "unknown_item"}})

    result = w.validate_equipment(e)
    assert len(result["issues"]) > 0
    reasons = [i["reason"] for i in result["issues"]]
    assert "item_not_registered" in reasons


def test_validate_equipment_empty(make_world):
    w = make_world()
    e = w.spawn_entity()
    w.set_component(e, "Equipment", {"slots": {}})

    result = w.validate_equipment(e)
    assert result["issues"] == []
