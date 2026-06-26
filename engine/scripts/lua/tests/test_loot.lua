-- test_loot.lua: Tests for the Rust-native loot table API.
-- Each test gets a fresh world via the test runner.
-- Global functions: define_loot_table(), roll_loot_table(), has_loot_table()

local assert = require("assert")

-- 1. Define table with 100% weight and roll returns items
local function test_define_and_roll_basic()
    local ok = define_loot_table("test_basic", {
        { item_id = "health_potion", weight = 100 },
    })
    assert.is_true(ok, "define_loot_table should return true")
    local results = roll_loot_table("test_basic")
    assert.is_true(#results > 0, "Should roll at least one item")
    assert.equals(results[1].item_id, "health_potion")
    assert.equals(results[1].count, 1)
end

-- 2. Rolling undefined table returns empty table without error
local function test_undefined_table_returns_empty()
    local results = roll_loot_table("nonexistent_table")
    assert.is_table(results)
    assert.equals(#results, 0)
end

-- 3. Multiple entries: weight=100 items always drop
local function test_multiple_entries_resolve()
    define_loot_table("test_multi", {
        { item_id = "health_potion", weight = 100 },
        { item_id = "rusty_sword", weight = 100 },
    })
    local results = roll_loot_table("test_multi")
    -- Both have 100% weight, so both should always drop
    assert.is_true(#results == 2, "Both entries should drop")
    local ids = {}
    for _, drop in ipairs(results) do
        ids[drop.item_id] = true
    end
    assert.is_true(ids["health_potion"], "health_potion should appear")
    assert.is_true(ids["rusty_sword"], "rusty_sword should appear")
end

-- 4. Weighted distribution: both items appear over many rolls
local function test_weighted_distribution()
    define_loot_table("test_weighted", {
        { item_id = "health_potion", weight = 90 },
        { item_id = "rusty_sword", weight = 10 },
    })
    local seen = {}
    for _ = 1, 100 do
        local results = roll_loot_table("test_weighted")
        for _, drop in ipairs(results) do
            seen[drop.item_id] = true
        end
    end
    assert.is_true(seen["health_potion"], "health_potion should appear in 100 rolls")
    assert.is_true(seen["rusty_sword"], "rusty_sword should appear in 100 rolls")
end

-- 5. min_count/max_count: count stays within range
local function test_min_max_count()
    define_loot_table("test_count", {
        { item_id = "coins", weight = 100, min_count = 2, max_count = 5 },
    })
    for _ = 1, 20 do
        local results = roll_loot_table("test_count")
        assert.is_true(#results >= 1, "Should drop at least one entry")
        if #results > 0 then
            local count = results[1].count
            assert.is_true(count >= 2 and count <= 5,
                "Count " .. count .. " should be in [2, 5]")
        end
    end
end

-- 6. Redefining a table overwrites previous entries
local function test_redefine_table_overwrites()
    define_loot_table("overwrite_test", {
        { item_id = "health_potion", weight = 100 },
    })
    define_loot_table("overwrite_test", {
        { item_id = "rusty_sword", weight = 100 },
    })
    local results = roll_loot_table("overwrite_test")
    assert.is_true(#results > 0)
    assert.equals(results[1].item_id, "rusty_sword",
        "Redefined table should drop rusty_sword")
end

-- 7. has_loot_table returns correct booleans
local function test_has_loot_table()
    assert.is_false(has_loot_table("unknown"), "Unknown table should return false")
    define_loot_table("known_table", {
        { item_id = "test", weight = 100 },
    })
    assert.is_true(has_loot_table("known_table"), "Defined table should return true")
end

-- 8. define_loot_table rejects zero-weight entries
local function test_zero_weight_rejected()
    local ok, err = pcall(define_loot_table, "bad", {
        { item_id = "test", weight = 0 },
    })
    assert.is_false(ok, "Zero-weight entry should be rejected")
end

-- 9. Define empty entries table succeeds, but rolling returns empty
local function test_empty_table_rolls_empty()
    local ok = define_loot_table("empty", {})
    assert.is_true(ok, "Empty entries table should be accepted")
    local results = roll_loot_table("empty")
    assert.is_table(results)
    assert.equals(#results, 0, "Rolling empty table should return empty")
end

-- 10. Multiple items from a single entry (min_count > 1)
local function test_multi_spawn_from_single_entry()
    define_loot_table("multi_spawn", {
        { item_id = "arrows", weight = 100, min_count = 3, max_count = 3 },
    })
    local results = roll_loot_table("multi_spawn")
    assert.is_true(#results >= 1, "Should drop at least one entry")
    if #results > 0 then
        assert.equals(results[1].item_id, "arrows")
        assert.equals(results[1].count, 3)
    end
end

return {
    test_define_and_roll_basic = test_define_and_roll_basic,
    test_undefined_table_returns_empty = test_undefined_table_returns_empty,
    test_multiple_entries_resolve = test_multiple_entries_resolve,
    test_weighted_distribution = test_weighted_distribution,
    test_min_max_count = test_min_max_count,
    test_redefine_table_overwrites = test_redefine_table_overwrites,
    test_has_loot_table = test_has_loot_table,
    test_zero_weight_rejected = test_zero_weight_rejected,
    test_empty_table_rolls_empty = test_empty_table_rolls_empty,
    test_multi_spawn_from_single_entry = test_multi_spawn_from_single_entry,
}
