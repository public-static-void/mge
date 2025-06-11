import pytest
import engine_py

def test_generate_and_apply_chunk(make_world):
    # Plugin is registered automatically at module init
    world = make_world()
    params = {
        "width": 2,
        "height": 2,
        "z_levels": 1,
        "seed": 123,
        "chunk_x": 0,
        "chunk_y": 0,
    }
    chunk = engine_py.invoke_worldgen_plugin("simple_square", params)
    assert "cells" in chunk and len(chunk["cells"]) == 4
    world.apply_generated_map(chunk)
    print("After first chunk:", world.get_map_cell_count())
    assert world.get_map_cell_count() == 4

    params2 = {
        "width": 2,
        "height": 2,
        "z_levels": 1,
        "seed": 456,
        "chunk_x": 2,
        "chunk_y": 0,
    }
    chunk2 = engine_py.invoke_worldgen_plugin("simple_square", params2)
    world.apply_chunk(chunk2)
    print("After second chunk:", world.get_map_cell_count())
    assert world.get_map_cell_count() == 8

def test_schema_validation_rejects_invalid_map(make_world):
    world = make_world()
    invalid_map = {"topology": "square", "cells": [{"x": 0, "y": 0}]}
    with pytest.raises(Exception):
        world.apply_generated_map(invalid_map)
