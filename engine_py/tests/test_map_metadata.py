def test_map_metadata(make_world):
    world = make_world()
    world.add_cell(1, 2, 0)
    world.set_cell_metadata(
        {"Square": {"x": 1, "y": 2, "z": 0}},
        {"biome": "Forest", "terrain": "Grass"},
    )
    meta = world.get_cell_metadata({"Square": {"x": 1, "y": 2, "z": 0}})
    assert meta["biome"] == "Forest"
    assert meta["terrain"] == "Grass"
