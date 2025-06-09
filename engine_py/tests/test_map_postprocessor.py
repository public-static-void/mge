import pytest

import engine_py


def test_register_and_invoke_map_postprocessor(make_world):
    world = make_world()
    called = {}

    def validator(world_obj):
        called["ran"] = True
        # Should be able to access map
        assert world_obj.get_map_cell_count() == 0 or isinstance(
            world_obj.get_map_cell_count(), int
        )
        return True

    world.register_map_postprocessor(validator)
    # Apply a generated map (should trigger postprocessor)
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    world.apply_generated_map(map_data)
    assert called.get("ran")


def test_postprocessor_can_block_apply(make_world):
    world = make_world()

    def always_fail(world_obj):
        raise Exception("fail on purpose")

    world.register_map_postprocessor(always_fail)
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    with pytest.raises(Exception) as excinfo:
        world.apply_generated_map(map_data)
    assert "fail on purpose" in str(excinfo.value)


def test_clear_map_postprocessors(make_world):
    world = make_world()
    called = {}

    def validator(world_obj):
        called["ran"] = True
        return True

    world.register_map_postprocessor(validator)
    world.clear_map_postprocessors()
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    world.apply_generated_map(map_data)
    assert not called.get("ran", False)
