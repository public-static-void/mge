import json

import mge as engine_py


def _params(seed):
    return {"seed": seed, "width": 20, "height": 15}


def _canonical(map_):
    return json.dumps(map_, sort_keys=True)


def test_builtin_algorithms_listed():
    names = engine_py.list_worldgen_plugins()
    assert "dungeon" in names
    assert "caves" in names


def test_switch_algorithms_by_name():
    dungeon_map = engine_py.invoke_worldgen_plugin("dungeon", _params(42))
    caves_map = engine_py.invoke_worldgen_plugin("caves", _params(42))
    assert dungeon_map["topology"] == "square"
    assert caves_map["topology"] == "square"
    assert len(dungeon_map["cells"]) > 0
    assert len(caves_map["cells"]) > 0


def test_caves_deterministic_for_fixed_seed():
    first = engine_py.invoke_worldgen_plugin("caves", _params(7))
    second = engine_py.invoke_worldgen_plugin("caves", _params(7))
    assert _canonical(first) == _canonical(second)


def test_caves_differs_across_seeds():
    seed_a = engine_py.invoke_worldgen_plugin("caves", _params(42))
    seed_b = engine_py.invoke_worldgen_plugin("caves", _params(99))
    assert _canonical(seed_a) != _canonical(seed_b)


def test_unknown_algorithm_names_candidates():
    try:
        engine_py.invoke_worldgen_plugin("no_such_algorithm", _params(42))
    except Exception as exc:
        message = str(exc)
        assert "dungeon" in message
        assert "caves" in message
    else:
        raise AssertionError("unknown algorithm name must raise")
