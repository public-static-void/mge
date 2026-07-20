"""Tests for the Python material system API bindings."""


def test_get_material_properties_wood(make_world):
    world = make_world()
    props = world.get_material_properties("wood")
    assert props is not None
    assert props["density"] == 0.6
    assert props["hardness"] == 2.0
    assert props["flammability"] == 0.9


def test_get_material_properties_unknown(make_world):
    world = make_world()
    result = world.get_material_properties("nonexistent")
    assert result is None


def test_get_material_names(make_world):
    world = make_world()
    names = world.get_material_names()
    expected = {"bone", "cloth", "iron", "leather", "obsidian", "steel", "stone", "wood"}
    assert set(names) == expected


def test_set_and_get_entity_material(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_entity_material(eid, "iron")
    result = world.get_entity_material(eid)
    assert result is not None
    assert result["material"] == "iron"
    assert result["quality"] == 1.0


def test_set_unknown_material_raises(make_world):
    world = make_world()
    import pytest
    eid = world.spawn_entity()
    with pytest.raises(ValueError):
        world.set_entity_material(eid, "nonexistent")


def test_get_entity_material_absent(make_world):
    world = make_world()
    eid = world.spawn_entity()
    result = world.get_entity_material(eid)
    assert result is None
