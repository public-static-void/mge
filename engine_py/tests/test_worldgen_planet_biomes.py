import json

import mge as engine_py


def _params(seed):
    return {"seed": seed, "width": 20, "height": 15}


def _canonical(map_):
    return json.dumps(map_, sort_keys=True)


def _assert_biome_shape(map_):
    assert map_["topology"] == "square"
    assert len(map_["cells"]) > 0
    for cell in map_["cells"]:
        assert isinstance(cell.get("biome"), str) and cell["biome"] != ""


def test_planet_biomes_listed():
    names = engine_py.list_worldgen_plugins()
    assert "planet_biomes" in names


def test_switch_to_planet_biomes_by_name():
    planet_map = engine_py.invoke_worldgen_plugin("planet_biomes", _params(42))
    _assert_biome_shape(planet_map)


def test_planet_biomes_deterministic_for_fixed_seed():
    first = engine_py.invoke_worldgen_plugin("planet_biomes", _params(7))
    second = engine_py.invoke_worldgen_plugin("planet_biomes", _params(7))
    assert _canonical(first) == _canonical(second)


def test_planet_biomes_differs_across_seeds():
    seed_a = engine_py.invoke_worldgen_plugin("planet_biomes", _params(42))
    seed_b = engine_py.invoke_worldgen_plugin("planet_biomes", _params(99))
    assert _canonical(seed_a) != _canonical(seed_b)


def test_planet_biomes_default_size_succeeds():
    defaulted = engine_py.invoke_worldgen_plugin("planet_biomes", {"seed": 42})
    _assert_biome_shape(defaulted)
    assert len(defaulted["cells"]) == 32 * 32
