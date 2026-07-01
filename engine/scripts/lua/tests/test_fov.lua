-- test_fov.lua: Tests for the field-of-view system.
-- Each test gets a fresh world via the test runner.
-- Global functions: spawn_entity, set_component, get_visible_cells, is_visible,
--                   set_sight, get_sight, set_position (via set_component)

local assert = require("assert")

-- Helper: create a simple open plane map for FOV testing
local function make_open_plane()
    local size = 10
    for x = -size, size do
        for y = -size, size do
            add_cell(x, y, 0)
        end
    end
    for x = -size, size do
        for y = -size, size do
            for dx = -1, 1 do
                for dy = -1, 1 do
                    if dx ~= 0 or dy ~= 0 then
                        local nx = x + dx
                        local ny = y + dy
                        if nx >= -size and nx <= size and ny >= -size and ny <= size then
                            add_neighbor({ x = x, y = y, z = 0 }, { x = nx, y = ny, z = 0 })
                        end
                    end
                end
            end
        end
    end
end

-- Helper: set position for an entity (schema requires {"pos": {"Square": {...}}})
local function set_position(eid, x, y, z)
    set_component(eid, "Position", { pos = { Square = { x = x, y = y, z = z } } })
end

-- 1. get_visible_cells returns a table for entity with Sight
local function test_get_visible_cells_basic()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    -- Run the FOV system
    tick()

    local cells = get_visible_cells(eid)
    assert.not_nil(cells, "get_visible_cells should return a table")
    local count = 0
    for _ in pairs(cells) do
        count = count + 1
    end
    assert.is_true(count > 0, "Visible cells count should be > 0")
end

-- 2. Origin cell is always visible
local function test_origin_is_visible()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    local visible = is_visible(eid, 0, 0, 0)
    assert.is_true(visible, "Origin cell should always be visible")
end

-- 3. set_sight sets the Sight component
local function test_set_sight()
    local eid = spawn_entity()
    set_sight(eid, 12)
    local sight = get_sight(eid)
    assert.not_nil(sight, "get_sight should return data after set_sight")
    assert.equals(sight.range, 12, "Sight range should be 12")
end

-- 4. get_sight returns nil for entity without Sight
local function test_get_sight_none()
    local eid = spawn_entity()
    local sight = get_sight(eid)
    assert.is_nil(sight, "get_sight should return nil for entity without Sight")
end

-- 5. set_sight -> get_sight round-trip
local function test_sight_roundtrip()
    local eid = spawn_entity()
    set_sight(eid, 8)
    local sight = get_sight(eid)
    assert.not_nil(sight)
    assert.equals(sight.range, 8, "Round-trip sight range should be 8")
end

-- 6. is_visible returns false for entity without Sight
local function test_is_visible_no_sight()
    make_open_plane()
    local eid = spawn_entity()
    set_position(eid, 0, 0, 0)
    tick()

    local visible = is_visible(eid, 0, 0, 0)
    assert.is_false(visible, "is_visible should return false for entity without Sight")
end

-- 7. get_visible_cells returns empty table for entity without Sight
local function test_get_visible_cells_no_sight()
    make_open_plane()
    local eid = spawn_entity()
    set_position(eid, 0, 0, 0)
    tick()

    local cells = get_visible_cells(eid)
    assert.not_nil(cells, "get_visible_cells should return a table even without Sight")
    local count = 0
    for _ in pairs(cells) do
        count = count + 1
    end
    assert.equals(count, 0, "Visible cells should be empty for entity without Sight")
end

return {
    test_get_visible_cells_basic = test_get_visible_cells_basic,
    test_origin_is_visible = test_origin_is_visible,
    test_set_sight = test_set_sight,
    test_get_sight_none = test_get_sight_none,
    test_sight_roundtrip = test_sight_roundtrip,
    test_is_visible_no_sight = test_is_visible_no_sight,
    test_get_visible_cells_no_sight = test_get_visible_cells_no_sight,
}
