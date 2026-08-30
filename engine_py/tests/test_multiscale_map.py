"""test_multiscale_map.py: Parity test for the multi-scale map navigation API.

Exercises all 9 functions: register_map, set_active_map, get_map_names,
get_active_map_name, link_maps, enter_map, exit_map, map_cell, unmap_cell.
Mirrors the behaviors asserted by the Rust core tests (AC001-AC012, AC019).
"""

import pytest


def square_map_json():
    return {
        "topology": "square",
        "cells": [
            {"x": 0, "y": 0, "z": 0},
            {"x": 1, "y": 0, "z": 0},
        ],
    }


def province_map_json():
    return {
        "topology": "province",
        "cells": [
            {"id": "prov_a"},
            {"id": "prov_b"},
        ],
    }


# AC001: two named maps registered, both in get_map_names().
def test_register_two_maps_appear_in_registry(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())

    names = world.get_map_names()
    assert "overmap" in names, "overmap should be registered"
    assert "field" in names, "field should be registered"
    assert len(names) == 2, "Exactly two maps registered"


# AC002: set_active_map reflects the selected map's topology.
def test_set_active_map_reflects_topology(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())

    world.set_active_map("field")
    assert world.get_map_topology_type() == "square", "field map should be square"

    world.set_active_map("overmap")
    assert world.get_map_topology_type() == "province", "overmap should be province"


# AC003: set_active_map on an unknown name errors and leaves active map unchanged.
def test_set_active_map_unknown_name_errors_and_unchanged(make_world):
    world = make_world()
    world.register_map("field", square_map_json())
    world.set_active_map("field")

    with pytest.raises(ValueError, match="not registered"):
        world.set_active_map("nonexistent")

    assert world.get_active_map_name() == "field", "Active map unchanged"
    assert world.get_map_topology_type() == "square", "Topology unchanged"


# AC004: get_active_map_name returns the last selected map; get_map_names
# contains exactly the registered names.
def test_active_map_name_and_exact_names(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())

    assert world.get_active_map_name() == "", "No map selected initially"

    world.set_active_map("field")
    assert world.get_active_map_name() == "field", "Active map is field"

    world.set_active_map("overmap")
    assert world.get_active_map_name() == "overmap", "Active map is overmap"

    names = world.get_map_names()
    assert len(names) == 2, "Exactly two registered names"
    assert "field" in names, "field in names"
    assert "overmap" in names, "overmap in names"


# AC005: a square-topology map can serve as an overmap (topology/type decoupled).
def test_square_topology_named_overmap(make_world):
    world = make_world()
    world.register_map("overmap", square_map_json())
    world.set_active_map("overmap")
    assert world.get_map_topology_type() == "square", "A square map named overmap is valid"


# AC006: a province-topology map can serve as a strategic map.
def test_province_topology_named_strategic(make_world):
    world = make_world()
    world.register_map("strategic", province_map_json())
    world.set_active_map("strategic")
    assert world.get_map_topology_type() == "province", "A province map named strategic is valid"


# AC007: enter_map switches the active map and positions the camera at the entry cell.
def test_enter_map_switches_active_and_positions_camera(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())
    world.set_active_map("overmap")

    world.link_maps(
        "overmap",
        {"Province": {"id": "prov_a"}},
        "field",
        {"Square": {"x": 1, "y": 0, "z": 0}},
    )

    world.enter_map("field", {"Square": {"x": 1, "y": 0, "z": 0}})

    assert world.get_active_map_name() == "field", "Active map is field"
    assert world.get_map_topology_type() == "square", "Field map is square"

    cam = world.get_camera()
    assert cam["x"] == 1, "Camera x at entry cell"
    assert cam["y"] == 0, "Camera y at entry cell"
    assert cam["z"] == 0, "Camera z at entry cell"


# AC008: exit_map returns to the previously active map.
def test_exit_map_returns_to_previous_map(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())
    world.set_active_map("overmap")

    world.enter_map("field", {"Square": {"x": 0, "y": 0, "z": 0}})
    assert world.get_active_map_name() == "field", "Active map is field after enter"

    world.exit_map()
    assert world.get_active_map_name() == "overmap", "Active map is overmap after exit"
    assert world.get_map_topology_type() == "province", "Overmap topology is province"


# AC009: enter_map on an unknown map errors; exit_map with an empty stack errors.
def test_enter_unknown_map_and_exit_empty_stack_error(make_world):
    world = make_world()
    world.register_map("field", square_map_json())

    with pytest.raises(ValueError, match="not registered"):
        world.enter_map("nonexistent", {"Square": {"x": 0, "y": 0, "z": 0}})

    with pytest.raises(ValueError, match="no previous map"):
        world.exit_map()


# AC010: map_cell/unmap_cell round-trip a linked source cell to its target cell and back.
def test_map_cell_unmap_cell_round_trip(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())

    source_cell = {"Province": {"id": "prov_a"}}
    target_cell = {"Square": {"x": 2, "y": 1, "z": 0}}
    world.link_maps("overmap", source_cell, "field", target_cell)

    assert world.map_cell("overmap", source_cell) == target_cell, (
        "map_cell returns the linked target cell"
    )
    assert world.unmap_cell("field", target_cell) == source_cell, (
        "unmap_cell returns the linked source cell"
    )


# AC011: map_cell/unmap_cell return None for unlinked source/target cells.
def test_map_cell_unmap_cell_unlinked_returns_none(make_world):
    world = make_world()
    world.register_map("overmap", province_map_json())
    world.register_map("field", square_map_json())

    world.link_maps(
        "overmap",
        {"Province": {"id": "prov_a"}},
        "field",
        {"Square": {"x": 2, "y": 1, "z": 0}},
    )

    assert world.map_cell("overmap", {"Province": {"id": "prov_b"}}) is None, (
        "Unlinked source cell returns None"
    )
    assert world.map_cell("field", {"Square": {"x": 0, "y": 0, "z": 0}}) is None, (
        "Unlinked map returns None"
    )
    assert world.unmap_cell("field", {"Square": {"x": 9, "y": 9, "z": 0}}) is None, (
        "Unlinked target cell returns None"
    )
    assert world.unmap_cell("overmap", {"Province": {"id": "prov_a"}}) is None, (
        "Unlinked target map returns None"
    )


# AC012: coordinate mapping is topology-generic (province -> square round-trip).
def test_cross_topology_round_trip_province_to_square(make_world):
    world = make_world()
    world.register_map("strategic", province_map_json())
    world.register_map("tactical", square_map_json())

    source_cell = {"Province": {"id": "prov_b"}}
    target_cell = {"Square": {"x": 3, "y": 4, "z": 0}}
    world.link_maps("strategic", source_cell, "tactical", target_cell)

    assert world.map_cell("strategic", source_cell) == target_cell, (
        "Province cell maps to square cell"
    )
    assert world.unmap_cell("tactical", target_cell) == source_cell, (
        "Square cell unmaps to province cell"
    )


# AC019: duplicate register_map errors (no panic).
def test_register_duplicate_name_errors(make_world):
    world = make_world()
    world.register_map("field", square_map_json())

    with pytest.raises(ValueError, match="already registered"):
        world.register_map("field", square_map_json())