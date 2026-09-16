import pytest


def test_zone_eight_op_sequence(make_world):
    world = make_world()
    world.set_mode("colony")

    zid = world.designate_zone(
        "stockpile", "depot", {"rect": {"x0": 0, "y0": 0, "z": 0, "x1": 1, "y1": 1}}
    )
    assert zid

    assert world.assign_cells_to_zone(zid, [{"Square": {"x": 5, "y": 5, "z": 0}}]) is True
    assert world.assign_cells_to_zone(zid, [{"Square": {"x": 5, "y": 5, "z": 0}}]) is True

    zones = world.list_zones()
    assert len(zones) == 1
    assert zones[0]["id"] == zid
    assert zones[0]["label"] == "depot"
    assert zones[0]["kind"] == "stockpile"
    assert zones[0]["cell_count"] == 5

    zone = world.get_zone(zid)
    assert zone is not None
    assert zone["id"] == zid
    assert zone["kind"] == "stockpile"

    assert world.rename_zone(zid, "store") is True
    assert world.set_zone_kind(zid, "farm") is True
    renamed = world.get_zone(zid)
    assert renamed["label"] == "store"
    assert renamed["kind"] == "farm"

    assert len(world.get_cells_in_region_kind("farm")) == 5
    assert world.get_cells_in_region_kind("stockpile") == []

    assert world.unassign_cells_from_zone(zid, [{"Square": {"x": 5, "y": 5, "z": 0}}]) is True
    assert world.unassign_cells_from_zone(zid, [{"Square": {"x": 9, "y": 9, "z": 0}}]) is True
    assert len(world.get_cells_in_region(zid)) == 4

    assert world.remove_zone(zid) is True
    assert world.get_zone(zid) is None
    assert world.remove_zone(zid) is False
    assert world.rename_zone(zid, "ghost") is False


def test_zone_cell_list_designate(make_world):
    world = make_world()
    world.set_mode("colony")

    zid = world.designate_zone(
        "farm",
        None,
        {"cells": [{"Square": {"x": 0, "y": 0, "z": 0}}, {"Square": {"x": 1, "y": 0, "z": 0}}]},
    )
    assert zid
    assert len(world.get_cells_in_region(zid)) == 2
    assert len(world.get_cells_in_region_kind("farm")) == 2


def test_zone_validation_rejects_bad_input(make_world):
    world = make_world()
    world.set_mode("colony")

    with pytest.raises(Exception):
        world.designate_zone("", None, {"rect": {"x0": 0, "y0": 0, "z": 0, "x1": 1, "y1": 1}})
    with pytest.raises(Exception):
        world.designate_zone(
            "farm", None, {"rect": {"x0": 2, "y0": 0, "z": 0, "x1": 1, "y1": 1}}
        )
    with pytest.raises(Exception):
        world.designate_zone("farm", None, {"cells": [{"bogus": 1}]})
    with pytest.raises(Exception):
        world.designate_zone("farm", None, {"blob": []})


def test_zone_mode_gate_outside_colony(make_world):
    world = make_world()
    world.set_mode("roguelike")

    with pytest.raises(Exception, match="colony"):
        world.designate_zone(
            "stockpile", None, {"rect": {"x0": 0, "y0": 0, "z": 0, "x1": 1, "y1": 1}}
        )
    with pytest.raises(Exception, match="colony"):
        world.remove_zone("zone-1")
    with pytest.raises(Exception, match="colony"):
        world.rename_zone("zone-1", "store")
    with pytest.raises(Exception, match="colony"):
        world.set_zone_kind("zone-1", "farm")
    with pytest.raises(Exception, match="colony"):
        world.assign_cells_to_zone("zone-1", [])
    with pytest.raises(Exception, match="colony"):
        world.unassign_cells_from_zone("zone-1", [])

    assert world.list_zones() == []
    assert world.get_zone("zone-1") is None
