-- test_lore.lua: Tests for the procedural history and lore chronicle bridge.
-- Each test gets a fresh world via the test runner.
-- Global functions: generate_founding_history, list_chronicle,
-- get_chronicle_entry, render_chronicle, chronicle_len, clear_lore_history.
-- The bridge only queries and backfills; live narrative mirroring lives in core.

local assert = require("assert")

-- 1. Backfill returns its count and the length mirrors the listing
local function test_backfill_count_matches_length()
    local count = generate_founding_history(42, 5)
    assert.equals(count, 5, "Backfill should report the appended count")
    assert.equals(chronicle_len(), 5, "Length should mirror the backfill count")
    local all = list_chronicle(nil)
    assert.is_table(all, "list_chronicle(nil) should return a table")
    assert.equals(#all, 5, "Nil filter should return every entry")
end

-- 2. Kind and scenario filters narrow the chronicle
local function test_kind_and_scenario_filters_narrow()
    generate_founding_history(42, 5)
    local founding = list_chronicle({ kind = "founding" })
    assert.equals(#founding, 5, "Kind filter should keep every founding entry")
    assert.equals(#list_chronicle({ kind = "fired" }), 0, "Unseen kind should match nothing")
    local first_id = founding[1].scenario_id
    local narrowed = list_chronicle({ scenario_id = first_id, kind = "founding" })
    assert.is_true(#narrowed >= 1, "Scenario filter should keep its own entries")
    for _, entry in ipairs(narrowed) do
        assert.equals(entry.scenario_id, first_id, "Narrowed entries should name the scenario")
        assert.equals(entry.kind, "founding", "Narrowed entries should carry the kind")
    end
end

-- 3. Turn-range filter keeps only entries inside the inclusive bounds
local function test_turn_range_filter_keeps_bounds()
    generate_founding_history(42, 5)
    local middle = list_chronicle({ turn_from = 1, turn_to = 2 })
    assert.equals(#middle, 2, "Turn range should keep exactly the bounded entries")
    assert.equals(middle[1].turn, 1, "Range should start at the lower bound")
    assert.equals(middle[2].turn, 2, "Range should end at the upper bound")
end

-- 4. Rendered lines follow the fixed chronicle template
local function test_render_follows_template()
    generate_founding_history(42, 5)
    local lines = render_chronicle(nil)
    assert.is_table(lines, "render_chronicle(nil) should return a table")
    assert.equals(#lines, 5, "Render should cover every entry")
    for _, line in ipairs(lines) do
        assert.is_true(
            string.find(line, "^Turn %d+: .+ founding$") ~= nil,
            "Rendered line should match the template: " .. tostring(line)
        )
    end
    local entries = list_chronicle(nil)
    assert.equals(lines[1], entries[1].summary, "Render should reuse the stored summary")
end

-- 5. Single-entry lookup returns the entry or nil
local function test_single_lookup_returns_entry_or_nil()
    generate_founding_history(42, 5)
    local entries = list_chronicle(nil)
    local one = get_chronicle_entry(entries[1].entry_id)
    assert.is_table(one, "Known id should return a table")
    assert.equals(one.entry_id, entries[1].entry_id, "Lookup should return the entry")
    assert.is_nil(get_chronicle_entry(9999), "Unknown id should return nil")
end

-- 6. Invalid kind strings raise instead of filtering
local function test_invalid_kind_raises()
    generate_founding_history(42, 5)
    local ok, err = pcall(list_chronicle, { kind = "bogus" })
    assert.is_false(ok, "Invalid list kind should error")
    assert.is_true(string.find(tostring(err) or "", "bogus") ~= nil, "Error should name the kind")
    local render_ok = pcall(render_chronicle, { kind = "bogus" })
    assert.is_false(render_ok, "Invalid render kind should error")
    assert.equals(chronicle_len(), 5, "Failed query should leave the chronicle untouched")
end

-- 7. Clearing resets the chronicle so backfill reproduces it
local function test_clear_resets_for_regen()
    generate_founding_history(42, 5)
    clear_lore_history()
    assert.equals(chronicle_len(), 0, "Clear should empty the chronicle")
    assert.equals(#list_chronicle(nil), 0, "Clear should empty the listing")
    local before = render_chronicle(nil)
    assert.equals(#before, 0, "Clear should empty the render")
    assert.equals(generate_founding_history(42, 5), 5, "Regen should append the same count")
    assert.equals(chronicle_len(), 5, "Regen should restore the length")
end

return {
    test_backfill_count_matches_length = test_backfill_count_matches_length,
    test_kind_and_scenario_filters_narrow = test_kind_and_scenario_filters_narrow,
    test_turn_range_filter_keeps_bounds = test_turn_range_filter_keeps_bounds,
    test_render_follows_template = test_render_follows_template,
    test_single_lookup_returns_entry_or_nil = test_single_lookup_returns_entry_or_nil,
    test_invalid_kind_raises = test_invalid_kind_raises,
    test_clear_resets_for_regen = test_clear_resets_for_regen,
}
