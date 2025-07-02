import pytest

def test_register_and_invoke_map_postprocessor(make_world):
    world = make_world()
    called = {}

    def validator(world_obj):
        called["ran"] = True
        assert world_obj.get_map_cell_count() == 0 or isinstance(
            world_obj.get_map_cell_count(), int
        )
        return True

    world.register_map_postprocessor(validator)
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

# === Map Validator Tests ===

def test_validator_blocks_invalid_map(make_world):
    world = make_world()
    world.clear_map_validators()

    def always_fail(map_obj):
        return False

    world.register_map_validator(always_fail)
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    with pytest.raises(Exception) as excinfo:
        world.apply_generated_map(map_data)
    assert "Map validator failed" in str(excinfo.value)

def test_validator_accepts_valid_map(make_world):
    world = make_world()
    world.clear_map_validators()
    called = {}

    def always_pass(map_obj):
        called["ran"] = True
        return True

    world.register_map_validator(always_pass)
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    world.apply_generated_map(map_data)
    assert called.get("ran")

def test_clear_validators(make_world):
    world = make_world()
    called = {}

    def validator(map_obj):
        called["ran"] = True
        return True

    world.register_map_validator(validator)
    world.clear_map_validators()
    map_data = {"topology": "square", "cells": [{"x": 0, "y": 0, "z": 0}]}
    world.apply_generated_map(map_data)
    assert not called.get("ran", False)
