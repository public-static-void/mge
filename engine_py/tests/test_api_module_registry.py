"""Bridge-parity registry tests: the Python bridge exposes every shared domain.

Mirrors the WASM ``wasm_api_module_registry`` suite: the pinned module list
is the drift guard, and the shared-domain superset assertion keeps the
Python bridge from silently dropping a canonical domain.
"""

import mge

SHARED_API_DOMAINS = [
    "body",
    "camera",
    "component",
    "construction",
    "craft",
    "death_decay",
    "diplomacy",
    "dungeon",
    "economic",
    "entity",
    "equipment",
    "event_bus",
    "faction",
    "fov",
    "input",
    "inventory",
    "job_ai",
    "job_board",
    "job_cancel",
    "job_events",
    "job_mutation",
    "job_query",
    "job_system",
    "lore",
    "loot",
    "map",
    "material",
    "mode",
    "movement_ops",
    "multiscale_map",
    "narrative",
    "noise",
    "region",
    "save_load",
    "supply",
    "system",
    "tech_tree",
    "temperature",
    "time_of_day",
    "trade",
    "turn",
    "ui",
    "unit_template",
    "vehicle",
    "weather",
    "worldgen",
]

EXPECTED_PY_MODULES = [
    "body",
    "camera",
    "component",
    "construction",
    "craft",
    "death_decay",
    "designer",
    "diplomacy",
    "dungeon",
    "economic",
    "entity",
    "equipment",
    "event_bus",
    "faction",
    "fov",
    "input",
    "inventory",
    "job_ai",
    "job_board",
    "job_cancel",
    "job_children",
    "job_dependencies",
    "job_events",
    "job_mutation",
    "job_production",
    "job_query",
    "job_reservation",
    "job_system",
    "lore",
    "loot",
    "map",
    "material",
    "mode",
    "movement_ops",
    "multiscale_map",
    "narrative",
    "noise",
    "region",
    "save_load",
    "supply",
    "system",
    "tech_tree",
    "temperature",
    "time_of_day",
    "trade",
    "turn",
    "ui",
    "unit_template",
    "vehicle",
    "weather",
    "world",
    "worldgen",
]


def test_registry_matches_pinned_module_list():
    assert mge.list_api_modules() == EXPECTED_PY_MODULES


def test_registry_covers_all_shared_domains():
    names = mge.list_api_modules()
    for domain in SHARED_API_DOMAINS:
        assert domain in names, f"shared domain '{domain}' missing from PY_API_MODULES"


def test_registry_names_are_unique():
    names = mge.list_api_modules()
    assert len(names) == len(set(names))
