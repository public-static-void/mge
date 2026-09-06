-- test_fluid.lua: Tests for the fluid simulation system across the Lua bridge.
-- Each test gets a fresh world via the test runner.
-- Global functions: add_cell, add_neighbor, set_cell_metadata, get_fluid, tick

local assert = require("assert")

-- Helper: create a 3x3 open plane map with all neighbors connected.
local function make_plane()
    for x = 0, 2 do
        for y = 0, 2 do
            add_cell(x, y, 0)
        end
    end
    for x = 0, 2 do
        for y = 0, 2 do
            for dx = -1, 1 do
                for dy = -1, 1 do
                    if dx ~= 0 or dy ~= 0 then
                        local nx = x + dx
                        local ny = y + dy
                        if nx >= 0 and nx <= 2 and ny >= 0 and ny <= 2 then
                            add_neighbor({ x = x, y = y, z = 0 }, { x = nx, y = ny, z = 0 })
                        end
                    end
                end
            end
        end
    end
end

-- 1. get_fluid returns nil for a cell with no fluid metadata
local function test_get_fluid_none()
    make_plane()
    local fluid = get_fluid({ x = 0, y = 0, z = 0 })
    assert.is_nil(fluid, "get_fluid should return nil for a cell without fluid")
end

-- 2. get_fluid returns the fluid value after set_cell_metadata
local function test_get_fluid_roundtrip()
    make_plane()
    set_cell_metadata(
        { x = 0, y = 0, z = 0 },
        { fluid = { type = "water", level = 5 } }
    )
    local fluid = get_fluid({ x = 0, y = 0, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table after set_cell_metadata")
    assert.equals(fluid.type, "water", "Fluid type should be water")
    assert.equals(fluid.level, 5, "Fluid level should be 5")
end

-- 3. get_fluid returns magma type correctly
local function test_get_fluid_magma()
    make_plane()
    set_cell_metadata(
        { x = 1, y = 1, z = 0 },
        { fluid = { type = "magma", level = 3 } }
    )
    local fluid = get_fluid({ x = 1, y = 1, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table for magma")
    assert.equals(fluid.type, "magma", "Fluid type should be magma")
    assert.equals(fluid.level, 3, "Fluid level should be 3")
end

-- 4. get_fluid returns dual-form (water + magma) correctly
local function test_get_fluid_dual()
    make_plane()
    set_cell_metadata(
        { x = 2, y = 2, z = 0 },
        { fluid = { water = 2, magma = 1 } }
    )
    local fluid = get_fluid({ x = 2, y = 2, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table for dual fluid")
    assert.equals(fluid.water, 2, "Water level should be 2")
    assert.equals(fluid.magma, 1, "Magma level should be 1")
end

-- 5. Fluid spreads to a neighbor after tick
local function test_fluid_spreads_after_tick()
    make_plane()
    -- Source cell with high water level
    set_cell_metadata(
        { x = 0, y = 0, z = 0 },
        { fluid = { type = "water", level = 8 } }
    )
    tick()
    -- Neighbor should now have some fluid
    local neighbor = get_fluid({ x = 1, y = 0, z = 0 })
    assert.not_nil(neighbor, "Neighbor should have fluid after tick")
    assert.is_true(neighbor.level > 0, "Neighbor fluid level should be > 0")
end

-- 6. Water cell defaults to fresh/shallow/flowing taxonomy
local function test_water_defaults_to_fresh_shallow_flowing()
    make_plane()
    set_cell_metadata(
        { x = 0, y = 0, z = 0 },
        { fluid = { type = "water", level = 4 } }
    )
    tick()
    local fluid = get_fluid({ x = 0, y = 0, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table")
    assert.equals(fluid.water_type, "fresh", "Default water_type should be fresh")
    assert.equals(fluid.depth, "shallow", "Default depth should be shallow")
    assert.equals(fluid.flow_state, "flowing", "Default flow_state should be flowing")
end

-- 7. Explicit taxonomy fields are preserved through a tick
local function test_taxonomy_fields_round_trip()
    make_plane()
    set_cell_metadata(
        { x = 0, y = 0, z = 0 },
        { fluid = { type = "water", level = 4, water_type = "salt", depth = "deep", flow_state = "stale" } }
    )
    tick()
    local fluid = get_fluid({ x = 0, y = 0, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table")
    assert.equals(fluid.water_type, "salt", "water_type should be preserved")
    assert.equals(fluid.depth, "deep", "depth should be preserved")
    assert.equals(fluid.flow_state, "stale", "flow_state should be preserved")
end

-- 8. Magma cells carry no taxonomy fields
local function test_magma_has_no_taxonomy()
    make_plane()
    set_cell_metadata(
        { x = 1, y = 1, z = 0 },
        { fluid = { type = "magma", level = 3 } }
    )
    tick()
    local fluid = get_fluid({ x = 1, y = 1, z = 0 })
    assert.not_nil(fluid, "get_fluid should return a table for magma")
    assert.equals(fluid.type, "magma", "Fluid type should be magma")
    assert.is_nil(fluid.water_type, "Magma should have no water_type")
    assert.is_nil(fluid.depth, "Magma should have no depth")
    assert.is_nil(fluid.flow_state, "Magma should have no flow_state")
end

-- 9. Mixing fresh water into salt water yields brackish
local function test_mixing_fresh_into_salt_yields_brackish()
    make_plane()
    -- Source: fresh level 8; receiver: salt level 1
    set_cell_metadata(
        { x = 0, y = 0, z = 0 },
        { fluid = { type = "water", level = 8, water_type = "fresh" } }
    )
    set_cell_metadata(
        { x = 1, y = 0, z = 0 },
        { fluid = { type = "water", level = 1, water_type = "salt" } }
    )
    tick()
    local receiver = get_fluid({ x = 1, y = 0, z = 0 })
    assert.not_nil(receiver, "Receiver should have fluid")
    assert.equals(receiver.water_type, "brackish", "Fresh into salt should yield brackish")
end

return {
    test_get_fluid_none = test_get_fluid_none,
    test_get_fluid_roundtrip = test_get_fluid_roundtrip,
    test_get_fluid_magma = test_get_fluid_magma,
    test_get_fluid_dual = test_get_fluid_dual,
    test_fluid_spreads_after_tick = test_fluid_spreads_after_tick,
    test_water_defaults_to_fresh_shallow_flowing = test_water_defaults_to_fresh_shallow_flowing,
    test_taxonomy_fields_round_trip = test_taxonomy_fields_round_trip,
    test_magma_has_no_taxonomy = test_magma_has_no_taxonomy,
    test_mixing_fresh_into_salt_yields_brackish = test_mixing_fresh_into_salt_yields_brackish,
}
