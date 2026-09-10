-- test_enemy_ai.lua: Tests for the AI behaviors API.
-- Each test gets a fresh world via the test runner.
-- Global functions: set_patrol_route, get_patrol_route, set_ai_state, get_ai_state

local assert = require("assert")

-- 1. set_patrol_route sets a PatrolRoute component
local function test_set_patrol_route()
    local id = spawn_entity()
    local result = set_patrol_route(id, {
        {x = 0, y = 0, z = 0},
        {x = 5, y = 0, z = 0}
    })
    assert.is_true(result, "set_patrol_route should return true")
end

-- 2. get_patrol_route returns waypoints after set_patrol_route
local function test_get_patrol_route_after_set()
    local id = spawn_entity()
    set_patrol_route(id, {
        {x = 0, y = 0, z = 0},
        {x = 5, y = 0, z = 0}
    })
    local route = get_patrol_route(id)
    assert.not_nil(route, "get_patrol_route should return a table")
    assert.not_nil(route.waypoints, "Route should have waypoints")
    assert.equals(route.waypoints[1].x, 0, "First waypoint x should be 0")
    assert.equals(route.waypoints[1].y, 0, "First waypoint y should be 0")
    assert.equals(route.waypoints[2].x, 5, "Second waypoint x should be 5")
    assert.equals(route.waypoints[2].y, 0, "Second waypoint y should be 0")
end

-- 3. get_patrol_route returns nil for entity without PatrolRoute
local function test_get_patrol_route_none()
    local id = spawn_entity()
    local route = get_patrol_route(id)
    assert.is_nil(route, "get_patrol_route should return nil for entity without PatrolRoute")
end

-- 4. patrol route round-trip preserves current_index, loop, wait_ticks
local function test_patrol_route_defaults()
    local id = spawn_entity()
    set_patrol_route(id, {
        {x = 1, y = 1, z = 0},
        {x = 2, y = 2, z = 0}
    })
    local route = get_patrol_route(id)
    assert.not_nil(route)
    assert.equals(route.current_index, 0, "current_index should default to 0")
    assert.equals(route["loop"], true, "loop should default to true")
    assert.equals(route.wait_ticks, 0, "wait_ticks should default to 0")
end

-- 5. set_ai_state sets state
local function test_set_ai_state()
    local id = spawn_entity()
    local result = set_ai_state(id, "patrol")
    assert.is_true(result, "set_ai_state should return true")
end

-- 6. get_ai_state returns full EnemyAI component after set_ai_state
local function test_get_ai_state_after_set()
    local id = spawn_entity()
    set_ai_state(id, "chase")
    local ai = get_ai_state(id)
    assert.not_nil(ai, "get_ai_state should return a table")
    assert.equals(ai.state, "chase", "State should be 'chase'")
    assert.equals(ai.detection_range, 8, "detection_range should default to 8")
    assert.equals(ai.attack_range, 1, "attack_range should default to 1")
end

-- 7. get_ai_state returns nil for entity without EnemyAI
local function test_get_ai_state_none()
    local id = spawn_entity()
    local ai = get_ai_state(id)
    assert.is_nil(ai, "get_ai_state should return nil for entity without EnemyAI")
end

-- 8. set_ai_state validates state string
local function test_set_ai_state_invalid()
    local id = spawn_entity()
    local ok, err = pcall(set_ai_state, id, "invalid_state")
    assert.is_false(ok, "set_ai_state with invalid state should error")
end

-- 9. set_ai_state creates default EnemyAI when none exists
local function test_set_ai_state_creates_default()
    local id = spawn_entity()
    set_ai_state(id, "idle")
    local ai = get_ai_state(id)
    assert.not_nil(ai, "get_ai_state should return a table after set_ai_state")
    assert.equals(ai.state, "idle", "State should be 'idle'")
    assert.equals(ai.detection_range, 8, "detection_range should be 8")
    assert.equals(ai.flee_threshold, 0.25, "flee_threshold should be 0.25")
end

-- 10. set_ai_state updates existing EnemyAI state
local function test_set_ai_state_updates()
    local id = spawn_entity()
    set_ai_state(id, "idle")
    set_ai_state(id, "flee")
    local ai = get_ai_state(id)
    assert.equals(ai.state, "flee", "State should be updated to 'flee'")
end

return {
    test_set_patrol_route = test_set_patrol_route,
    test_get_patrol_route_after_set = test_get_patrol_route_after_set,
    test_get_patrol_route_none = test_get_patrol_route_none,
    test_patrol_route_defaults = test_patrol_route_defaults,
    test_set_ai_state = test_set_ai_state,
    test_get_ai_state_after_set = test_get_ai_state_after_set,
    test_get_ai_state_none = test_get_ai_state_none,
    test_set_ai_state_invalid = test_set_ai_state_invalid,
    test_set_ai_state_creates_default = test_set_ai_state_creates_default,
    test_set_ai_state_updates = test_set_ai_state_updates,
}
