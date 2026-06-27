-- Dungeon Generation Tests — Lua binding for Rust-native generate_dungeon()
local assert = require("assert")

array_mt = { __is_array = true }

local function deep_equal(a, b)
    if type(a) ~= type(b) then
        return false
    end
    if type(a) == "table" then
        local a_keys = {}
        for k, _ in pairs(a) do
            a_keys[k] = true
        end
        for k, _ in pairs(b) do
            if not a_keys[k] then
                return false
            end
            a_keys[k] = nil
        end
        if next(a_keys) then
            return false
        end
        for k, v in pairs(a) do
            if not deep_equal(v, b[k]) then
                return false
            end
        end
        return true
    end
    return a == b
end

local function test_generates_valid_map()
    local result = generate_dungeon({ width = 40, height = 25, seed = 42 })

    assert.is_table(result, "Result should be a table")
    assert.equals(result.topology, "square", "Topology should be square")
    assert.is_table(result.cells, "Cells should be a table")
    assert.equals(#result.cells, 1000, "Map should have 1000 cells")

    -- Verify cell structure
    local first_cell = result.cells[1]
    assert.equals(type(first_cell.x), "number", "Cell should have numeric x")
    assert.equals(type(first_cell.y), "number", "Cell should have numeric y")
    assert.equals(type(first_cell.z), "number", "Cell should have numeric z")

    -- Should have some walkable cells
    local walkable_count = 0
    local wall_count = 0
    for _, cell in ipairs(result.cells) do
        if cell.metadata and cell.metadata.walkable == false then
            wall_count = wall_count + 1
        else
            walkable_count = walkable_count + 1
        end
    end
    assert.is_true(walkable_count > 0, "Map should have walkable cells")
    assert.is_true(wall_count > 0, "Map should have wall cells")
end

local function test_same_seed_identical()
    local a = generate_dungeon({ width = 40, height = 25, seed = 42 })
    local b = generate_dungeon({ width = 40, height = 25, seed = 42 })

    assert.is_true(deep_equal(a, b), "Same seed should produce identical maps")
end

local function test_different_seeds_different()
    local a = generate_dungeon({ width = 40, height = 25, seed = 1 })
    local b = generate_dungeon({ width = 40, height = 25, seed = 2 })

    -- Convert cells to walkable arrays for comparison
    local a_walkable = {}
    local b_walkable = {}
    for i, cell in ipairs(a.cells) do
        a_walkable[i] = cell.metadata and cell.metadata.walkable == false and 0 or 1
    end
    for i, cell in ipairs(b.cells) do
        b_walkable[i] = cell.metadata and cell.metadata.walkable == false and 0 or 1
    end

    local same = true
    for i = 1, #a_walkable do
        if a_walkable[i] ~= b_walkable[i] then
            same = false
            break
        end
    end
    assert.is_false(same, "Different seeds should produce different maps")
end

local function test_invalid_config_error()
    local ok, err = pcall(generate_dungeon, { width = 0, height = 0 })
    assert.is_false(ok, "generate_dungeon with zero dimensions should raise an error: " .. tostring(err))
end

local function test_max_rooms_zero()
    local result = generate_dungeon({ width = 40, height = 25, seed = 42, max_rooms = 0 })

    -- All-wall map: every cell should have walkable=false metadata
    local walkable_count = 0
    for _, cell in ipairs(result.cells) do
        if not (cell.metadata and cell.metadata.walkable == false) then
            walkable_count = walkable_count + 1
        end
    end
    assert.equals(walkable_count, 0, "max_rooms=0 should produce all-wall map")
end

local function test_min_greater_than_max()
    local ok, result = pcall(generate_dungeon, {
        width = 40, height = 25, seed = 42,
        min_room_size = 10, max_room_size = 3
    })
    assert.is_true(ok, "min > max room size should not crash")
    if ok then
        local walkable = 0
        for _, cell in ipairs(result.cells) do
            if not (cell.metadata and cell.metadata.walkable == false) then
                walkable = walkable + 1
            end
        end
        assert.is_true(walkable > 0, "Should still produce walkable cells")
    end
end

return {
    test_generates_valid_map = test_generates_valid_map,
    test_same_seed_identical = test_same_seed_identical,
    test_different_seeds_different = test_different_seeds_different,
    test_invalid_config_error = test_invalid_config_error,
    test_max_rooms_zero = test_max_rooms_zero,
    test_min_greater_than_max = test_min_greater_than_max,
}
