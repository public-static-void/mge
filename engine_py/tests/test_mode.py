import os

import mge


def make_world():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    return mge.PyWorld(schema_dir)


def test_mode_management():
    world = make_world()
    # Default mode should be "colony"
    assert world.get_mode() == "colony"
    # Set mode to "roguelike"
    world.set_mode("roguelike")
    assert world.get_mode() == "roguelike"
    # Available modes should include both
    modes = world.get_available_modes()
    assert "colony" in modes
    assert "roguelike" in modes
