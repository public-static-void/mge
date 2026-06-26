"""Tests for the Python loot table API bindings."""


def test_define_and_roll(make_world):
    world = make_world()
    result = world.define_loot_table("test", [
        {"item_id": "health_potion", "weight": 100}
    ])
    assert result is True
    drops = world.roll_loot_table("test")
    assert len(drops) == 1
    assert drops[0]["item_id"] == "health_potion"
    assert drops[0]["count"] == 1


def test_define_with_min_max_count(make_world):
    world = make_world()
    result = world.define_loot_table("multi", [
        {"item_id": "coins", "weight": 100, "min_count": 2, "max_count": 5}
    ])
    assert result is True
    for _ in range(20):
        drops = world.roll_loot_table("multi")
        assert len(drops) == 1
        assert 2 <= drops[0]["count"] <= 5


def test_multiple_entries(make_world):
    world = make_world()
    world.define_loot_table("mixed", [
        {"item_id": "common", "weight": 90},
        {"item_id": "rare", "weight": 10},
    ])
    drops = world.roll_loot_table("mixed")
    assert 1 <= len(drops) <= 2


def test_has_loot_table(make_world):
    world = make_world()
    assert world.has_loot_table("nonexistent") is False
    world.define_loot_table("foo", [{"item_id": "bar", "weight": 100}])
    assert world.has_loot_table("foo") is True


def test_loot_table_names(make_world):
    world = make_world()
    world.define_loot_table("a", [{"item_id": "x", "weight": 100}])
    world.define_loot_table("b", [{"item_id": "y", "weight": 100}])
    names = world.loot_table_names()
    assert len(names) == 2
    assert "a" in names
    assert "b" in names


def test_remove_loot_table(make_world):
    world = make_world()
    world.define_loot_table("temp", [{"item_id": "x", "weight": 100}])
    assert world.has_loot_table("temp") is True
    world.remove_loot_table("temp")
    assert world.has_loot_table("temp") is False


def test_roll_nonexistent_table(make_world):
    world = make_world()
    import pytest
    with pytest.raises(ValueError, match="not found"):
        world.roll_loot_table("nonexistent")


def test_define_zero_weight_raises(make_world):
    world = make_world()
    import pytest
    with pytest.raises(ValueError, match="zero weight"):
        world.define_loot_table("bad", [
            {"item_id": "item", "weight": 0}
        ])


def test_define_min_gt_max_raises(make_world):
    world = make_world()
    import pytest
    with pytest.raises(ValueError):
        world.define_loot_table("bad", [
            {"item_id": "x", "weight": 100, "min_count": 5, "max_count": 1}
        ])


def test_define_overwrites(make_world):
    world = make_world()
    world.define_loot_table("dupe", [{"item_id": "old", "weight": 100}])
    world.define_loot_table("dupe", [{"item_id": "new", "weight": 100}])
    drops = world.roll_loot_table("dupe")
    assert drops[0]["item_id"] == "new"


def test_empty_table(make_world):
    world = make_world()
    world.define_loot_table("empty", [])
    import pytest
    with pytest.raises(ValueError, match="no entries"):
        world.roll_loot_table("empty")


def test_missing_item_id_raises(make_world):
    world = make_world()
    import pytest
    with pytest.raises((ValueError, TypeError)):
        world.define_loot_table("bad", [{"weight": 100}])
