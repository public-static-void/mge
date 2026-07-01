-- test_fog.lua: Tests for the fog-of-war system.
-- Each test gets a fresh world via the test runner.
-- Global functions: spawn_entity, set_component, add_cell, add_neighbor,
--                   tick, is_explored, get_explored_cells, reset_fog,
--                   get_visibility_state, set_sight

local assert = require("assert")

-- Helper: create a simple open plane map
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

-- Helper: set position for an entity
local function set_position(eid, x, y, z)
    set_component(eid, "Position", { pos = { Square = { x = x, y = y, z = z } } })
end

-- 1. is_explored returns true for cells visible after tick
local function test_is_explored_after_tick()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    local explored = is_explored(eid, 0, 0, 0)
    assert.is_true(explored, "Origin should be explored after tick")
end

-- 2. is_explored returns false for cells never seen
local function test_is_explored_false_for_unseen()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    local explored = is_explored(eid, 99, 99, 0)
    assert.is_false(explored, "Far-away cell should not be explored")
end

-- 3. reset_fog clears explored state
local function test_reset_fog()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    assert.is_true(is_explored(eid, 0, 0, 0), "Origin should be explored before reset")
    reset_fog(eid)
    assert.is_false(is_explored(eid, 0, 0, 0), "Origin should not be explored after reset")
end

-- 4. get_explored_cells returns non-empty after tick
local function test_get_explored_cells_after_tick()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    local cells = get_explored_cells(eid)
    assert.not_nil(cells, "get_explored_cells should return a table")
    local count = 0
    for _ in pairs(cells) do
        count = count + 1
    end
    assert.is_true(count > 0, "Explored cells count should be > 0")
end

-- 5. get_visibility_state returns correct values
local function test_get_visibility_state()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 5)
    set_position(eid, 0, 0, 0)
    tick()

    local origin_state = get_visibility_state(eid, 0, 0, 0)
    assert.equals(origin_state, 2, "Origin should be VISIBLE (2)")

    local unseen_state = get_visibility_state(eid, 99, 99, 0)
    assert.equals(unseen_state, 0, "Far cell should be UNEXPLORED (0)")
end

-- 6. Fog state accumulates across multiple ticks
local function test_fog_accumulation()
    make_open_plane()
    local eid = spawn_entity()
    set_sight(eid, 3)
    set_position(eid, 0, 0, 0)
    tick()

    -- Cell at distance 3 should be explored
    assert.is_true(is_explored(eid, 3, 0, 0), "Cell (3,0) should be explored")

    -- Move away — cell should remain explored
    set_position(eid, 10, 10, 0)
    tick()

    assert.is_true(is_explored(eid, 3, 0, 0), "Cell (3,0) should remain explored after moving")
end

return {
    test_is_explored_after_tick = test_is_explored_after_tick,
    test_is_explored_false_for_unseen = test_is_explored_false_for_unseen,
    test_reset_fog = test_reset_fog,
    test_get_explored_cells_after_tick = test_get_explored_cells_after_tick,
    test_get_visibility_state = test_get_visibility_state,
    test_fog_accumulation = test_fog_accumulation,
}
