"""Bridge parity tests for procedural history and lore chronicle.

Covers the identical six-function surface through the Python bridge:
generate_founding_history, list_chronicle, get_chronicle_entry,
render_chronicle, chronicle_len, clear_lore_history. Backfill generation
and the chronicle read-model live in Rust core; the bridge only queries
and backfills.
"""

import re

import pytest


def test_backfill_count_matches_length(make_world):
    world = make_world()
    count = world.generate_founding_history(42, 5)
    assert count == 5
    assert world.chronicle_len() == 5
    all_entries = world.list_chronicle(None)
    assert isinstance(all_entries, list)
    assert len(all_entries) == 5
    assert len(world.list_chronicle({})) == 5


def test_kind_and_scenario_filters_narrow(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    founding = world.list_chronicle({"kind": "founding"})
    assert len(founding) == 5
    assert world.list_chronicle({"kind": "fired"}) == []
    first_id = founding[0]["scenario_id"]
    narrowed = world.list_chronicle({"scenario_id": first_id, "kind": "founding"})
    assert len(narrowed) >= 1
    for entry in narrowed:
        assert entry["scenario_id"] == first_id
        assert entry["kind"] == "founding"


def test_turn_range_filter_keeps_bounds(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    middle = world.list_chronicle({"turn_from": 1, "turn_to": 2})
    assert len(middle) == 2
    assert middle[0]["turn"] == 1
    assert middle[1]["turn"] == 2


def test_render_follows_template(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    lines = world.render_chronicle(None)
    assert isinstance(lines, list)
    assert len(lines) == 5
    for line in lines:
        assert isinstance(line, str)
        assert re.match(r"^Turn \d+: .+ founding$", line), (
            f"Rendered line should match the template: {line}"
        )
    entries = world.list_chronicle(None)
    assert lines[0] == entries[0]["summary"]


def test_single_lookup_returns_entry_or_none(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    entries = world.list_chronicle(None)
    one = world.get_chronicle_entry(entries[0]["entry_id"])
    assert isinstance(one, dict)
    assert one["entry_id"] == entries[0]["entry_id"]
    assert world.get_chronicle_entry(9999) is None


def test_invalid_kind_raises(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    with pytest.raises(ValueError, match="bogus"):
        world.list_chronicle({"kind": "bogus"})
    with pytest.raises(ValueError, match="bogus"):
        world.render_chronicle({"kind": "bogus"})
    assert world.chronicle_len() == 5


def test_clear_resets_for_regen(make_world):
    world = make_world()
    world.generate_founding_history(42, 5)
    world.clear_lore_history()
    assert world.chronicle_len() == 0
    assert world.list_chronicle(None) == []
    assert world.render_chronicle(None) == []
    assert world.generate_founding_history(42, 5) == 5
    assert world.chronicle_len() == 5
