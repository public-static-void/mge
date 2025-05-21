import pytest

import mge


def test_register_and_invoke_worldgen():
    def pygen(params):
        assert isinstance(params, dict)
        assert params.get("width") == 5
        return {"cells": [{"id": "pycell", "x": 0, "y": 0}]}

    mge.register_worldgen_plugin("pygen", pygen)
    names = mge.list_worldgen_plugins()
    assert "pygen" in names

    result = mge.invoke_worldgen_plugin("pygen", {"width": 5})
    assert "cells" in result
    assert result["cells"][0]["id"] == "pycell"


def test_register_and_list_worldgen_plugins():
    def pygen_list(params):
        assert isinstance(params, dict)
        return {"topology": "square", "cells": [{"x": 0, "y": 0}]}

    mge.register_worldgen_plugin("pygen_list", pygen_list)
    plugins = mge.list_worldgen_plugins()
    assert "pygen_list" in plugins


def test_invoke_worldgen_plugin():
    def pygen2(params):
        w = params.get("width", 1)
        h = params.get("height", 1)
        cells = [{"x": x, "y": y} for x in range(w) for y in range(h)]
        return {"topology": "square", "cells": cells}

    mge.register_worldgen_plugin("pygen2", pygen2)
    result = mge.invoke_worldgen_plugin("pygen2", {"width": 2, "height": 2})
    assert result["topology"] == "square"
    assert isinstance(result["cells"], list)
    assert len(result["cells"]) == 4
    assert {"x": 0, "y": 0} in result["cells"]
    assert {"x": 1, "y": 1} in result["cells"]


def test_invoke_nonexistent_plugin_raises():
    with pytest.raises(Exception) as excinfo:
        mge.invoke_worldgen_plugin("nope", {})
    assert (
        "NotFound" in str(excinfo.value)
        or "not found" in str(excinfo.value).lower()
    )
