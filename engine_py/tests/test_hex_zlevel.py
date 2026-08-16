def _contains_hex_cell(cells, q, r, z):
    """Search a list of CellKey dicts (e.g. {"Hex": {"q": 1, "r": 2, "z": 3}}) for a Hex cell."""
    for cell in cells:
        if cell.get("Hex") == {"q": q, "r": r, "z": z}:
            return True
    return False


def _apply_hex_map(world, cells):
    world.apply_generated_map({"topology": "hex", "cells": cells})


def test_hex_add_cell(make_world):
    world = make_world()
    _apply_hex_map(world, [{"q": 0, "r": 0, "z": 0}])
    assert world.get_map_topology_type() == "hex"

    world.add_cell(1, 2, 3)

    cells = world.get_all_cells()
    assert _contains_hex_cell(cells, 0, 0, 0), "apply_generated_map should have added Hex(0, 0, 0)"
    assert _contains_hex_cell(cells, 1, 2, 3), "add_cell should add a Hex cell as (q, r, z)"


def test_hex_add_neighbor(make_world):
    world = make_world()
    # Cells are not 6-adjacent, so no neighbor is inferred: the explicit
    # add_neighbor edge is the only way the pair can appear.
    _apply_hex_map(world, [{"q": 0, "r": 0, "z": 0}, {"q": 5, "r": 5, "z": 0}])

    world.add_neighbor((0, 0, 0), (5, 5, 0))

    neighbors = world.get_neighbors({"Hex": {"q": 0, "r": 0, "z": 0}})
    assert _contains_hex_cell(neighbors, 5, 5, 0), "add_neighbor should add a hex neighbor edge"


def test_hex_entities_in_zlevel(make_world):
    world = make_world()
    _apply_hex_map(world, [{"q": 0, "r": 0, "z": 0}, {"q": 2, "r": 0, "z": 1}])

    lower = world.spawn_entity()
    world.set_component(lower, "Position", {"pos": {"Hex": {"q": 0, "r": 0, "z": 0}}})
    upper = world.spawn_entity()
    world.set_component(upper, "Position", {"pos": {"Hex": {"q": 2, "r": 0, "z": 1}}})

    on_zero = world.entities_in_zlevel(0)
    assert lower in on_zero, "Lower entity should be on z-level 0"
    assert upper not in on_zero, "Upper entity should not be on z-level 0"

    on_one = world.entities_in_zlevel(1)
    assert upper in on_one, "Upper entity should be on z-level 1"
    assert lower not in on_one, "Lower entity should not be on z-level 1"


def test_hex_move_entity_3d(make_world):
    world = make_world()
    _apply_hex_map(world, [{"q": 1, "r": 1, "z": 0}])

    eid = world.spawn_entity()
    world.set_component(eid, "Position", {"pos": {"Hex": {"q": 1, "r": 1, "z": 0}}})

    world.move_entity_3d(eid, 2, 3, 1)

    pos = world.get_component(eid, "Position")
    assert pos["pos"]["Hex"]["q"] == 3, "q should shift by dx"
    assert pos["pos"]["Hex"]["r"] == 4, "r should shift by dy"
    assert pos["pos"]["Hex"]["z"] == 1, "z should shift by dz"

    assert eid in world.entities_in_zlevel(1), "Entity should be found on new z-level"


def test_hex_camera(make_world):
    world = make_world()
    _apply_hex_map(world, [{"q": 0, "r": 0, "z": 0}])

    world.set_camera(3, 7, 2)

    cam = world.get_camera()
    assert cam["x"] == 3, "Camera x should be 3 after set_camera on a hex map"
    assert cam["y"] == 7, "Camera y should be 7 after set_camera on a hex map"
    assert cam["z"] == 2, "Camera z should be 2 after set_camera on a hex map"

    camera_id = world.get_entities_with_component("Camera")[0]
    pos = world.get_component(camera_id, "Position")
    assert pos["pos"]["Hex"]["q"] == 3, "Position should store pos.Hex.q on a hex map"
    assert pos["pos"]["Hex"]["r"] == 7, "Position should store pos.Hex.r on a hex map"
    assert pos["pos"]["Hex"]["z"] == 2, "Position should store pos.Hex.z on a hex map"
