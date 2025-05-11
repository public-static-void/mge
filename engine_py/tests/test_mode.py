def test_mode_management(make_world):
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
