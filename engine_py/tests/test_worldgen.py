import pytest


def test_register_and_invoke_worldgen(make_world):
    # Register a Python worldgen function
    def pygen(params):
        # params is a dict
        assert isinstance(params, dict)
        assert params.get("width") == 5
        return {"cells": [{"id": "pycell", "x": 0, "y": 0}]}

    world = make_world()
    world.register_worldgen("pygen", pygen)

    # It should appear in the worldgen list
    names = world.list_worldgen()
    assert "pygen" in names

    # Invocation should call our function and return the expected structure
    result = world.invoke_worldgen("pygen", {"width": 5})
    assert "cells" in result
    assert result["cells"][0]["id"] == "pycell"
