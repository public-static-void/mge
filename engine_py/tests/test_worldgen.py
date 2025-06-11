import pytest

import engine_py


def test_register_and_invoke_worldgen():
    def pygen(params):
        assert isinstance(params, dict)
        assert params.get("width") == 5
        # Return a valid square map (no "id", must have "topology", "x", "y", "z", "neighbors")
        return {
            "topology": "square",
            "cells": [{"x": 0, "y": 0, "z": 0, "neighbors": []}],
        }

    engine_py.register_worldgen_plugin("pygen", pygen)
    names = engine_py.list_worldgen_plugins()
    assert "pygen" in names

    result = engine_py.invoke_worldgen_plugin("pygen", {"width": 5})
    assert "cells" in result
    assert result["topology"] == "square"
    cell = result["cells"][0]
    assert cell["x"] == 0
    assert cell["y"] == 0
    assert cell["z"] == 0
    assert isinstance(cell["neighbors"], list)


def test_register_and_list_worldgen_plugins():
    def pygen_list(params):
        assert isinstance(params, dict)
        # Must return a valid map
        return {
            "topology": "square",
            "cells": [{"x": 0, "y": 0, "z": 0, "neighbors": []}],
        }

    engine_py.register_worldgen_plugin("pygen_list", pygen_list)
    plugins = engine_py.list_worldgen_plugins()
    assert "pygen_list" in plugins


def test_invoke_worldgen_plugin():
    def pygen2(params):
        w = params.get("width", 1)
        h = params.get("height", 1)
        # Cells must have x, y, z, neighbors for square topology
        cells = [
            {"x": x, "y": y, "z": 0, "neighbors": []}
            for x in range(w)
            for y in range(h)
        ]
        return {"topology": "square", "cells": cells}

    engine_py.register_worldgen_plugin("pygen2", pygen2)
    result = engine_py.invoke_worldgen_plugin(
        "pygen2", {"width": 2, "height": 2}
    )
    assert result["topology"] == "square"
    assert isinstance(result["cells"], list)
    assert len(result["cells"]) == 4
    for cell in result["cells"]:
        assert (
            "x" in cell and "y" in cell and "z" in cell and "neighbors" in cell
        )
        assert cell["z"] == 0
        assert isinstance(cell["neighbors"], list)


def test_invoke_nonexistent_plugin_raises():
    with pytest.raises(Exception) as excinfo:
        engine_py.invoke_worldgen_plugin("nope", {})
    assert (
        "NotFound" in str(excinfo.value)
        or "not found" in str(excinfo.value).lower()
    )
