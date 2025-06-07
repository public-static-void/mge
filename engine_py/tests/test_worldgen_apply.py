import mge


def test_apply_generated_map(make_world):
    # Register built-in plugins
    mge.register_builtin_worldgen_plugins_py()
    world = make_world()
    # Generate map using a built-in or test plugin
    map = mge.invoke_worldgen_plugin(
        "basic_square_worldgen",
        {"width": 4, "height": 4, "z_levels": 1, "seed": 42},
    )
    # Apply it to the world instance (not mge.world!)
    world.apply_generated_map(map)
    # Check that the world has a map and it has expected properties
    assert world.get_map_topology_type() == "square"
    assert world.get_map_cell_count() == 16
