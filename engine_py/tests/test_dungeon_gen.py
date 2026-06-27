"""Tests for procedural dungeon generation Python binding."""

import mge as engine_py


def test_generates_valid_map():
    """AC010: generate_dungeon returns dict with cells and neighbors."""
    world = engine_py.make_world("../../engine/assets/schemas")
    result = world.generate_dungeon({"width": 40, "height": 25, "seed": 42})

    assert "topology" in result
    assert result["topology"] == "square"
    assert "cells" in result
    assert len(result["cells"]) == 1000

    # Verify wall cells have metadata
    wall_count = 0
    floor_count = 0
    for cell in result["cells"]:
        assert "x" in cell
        assert "y" in cell
        assert "z" in cell
        meta = cell.get("metadata", {})
        if meta.get("walkable") is False:
            wall_count += 1
        else:
            floor_count += 1

    assert floor_count > 0, "Map should have floor cells"
    assert wall_count > 0, "Map should have wall cells"


def test_invalid_config_raises():
    """AC011: generate_dungeon with zero dimensions raises ValueError."""
    world = engine_py.make_world("../../engine/assets/schemas")
    try:
        world.generate_dungeon({"width": 0, "height": 0})
        assert False, "Should have raised ValueError"
    except ValueError as e:
        assert "positive" in str(e).lower()


def test_same_seed_identical():
    """Same seed + config produces identical maps."""
    world = engine_py.make_world("../../engine/assets/schemas")
    a = world.generate_dungeon({"width": 40, "height": 25, "seed": 42})
    b = world.generate_dungeon({"width": 40, "height": 25, "seed": 42})

    # Compare cell walkable states
    a_walkable = [cell.get("metadata", {}).get("walkable") for cell in a["cells"]]
    b_walkable = [cell.get("metadata", {}).get("walkable") for cell in b["cells"]]
    assert a_walkable == b_walkable


def test_different_seeds_different():
    """Different seeds produce different layouts."""
    world = engine_py.make_world("../../engine/assets/schemas")
    a = world.generate_dungeon({"width": 40, "height": 25, "seed": 1})
    b = world.generate_dungeon({"width": 40, "height": 25, "seed": 99})

    a_walkable = [1 if cell.get("metadata", {}).get("walkable") is False else 0 for cell in a["cells"]]
    b_walkable = [1 if cell.get("metadata", {}).get("walkable") is False else 0 for cell in b["cells"]]
    assert a_walkable != b_walkable, "Different seeds should produce different maps"
