import engine_py


def test_apply_generated_map(make_world):
    # Plugin is registered automatically at module init
    world = make_world()
    # Generate map using the C plugin
    map = engine_py.invoke_worldgen_plugin(
        "simple_square",
        {"width": 4, "height": 4, "z_levels": 1, "seed": 42},
    )
    world.apply_generated_map(map)
    assert world.get_map_topology_type() == "square"
    assert world.get_map_cell_count() == 16
