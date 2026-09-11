-- test_noise.lua: Tests for the noise and detection system.
-- Each test gets a fresh world via the test runner.
-- Global functions: spawn_entity, set_component, emit_noise, get_noise_at,
--                   set_hearing, get_hearing, set_ai_state, get_ai_state, tick

local assert = require("assert")

-- Helper: create a simple open plane map for noise testing
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

-- 1. emit_noise + get_noise_at round-trip
local function test_emit_noise_get_noise_at()
    make_open_plane()
    local eid = spawn_entity()
    set_position(eid, 0, 0, 0)
    local ok = emit_noise(eid, 1.0, 5)
    assert.is_true(ok, "emit_noise should return true")

    tick()

    assert.equals(get_noise_at(0, 0, 0), 1.0, "Origin should receive full intensity")
    assert.equals(get_noise_at(1, 0, 0), 0.8, "Distance 1 should receive 0.8")
    assert.equals(get_noise_at(2, 0, 0), 0.6, "Distance 2 should receive 0.6")
    assert.equals(get_noise_at(6, 0, 0), 0.0, "Beyond radius should be silent")
end

-- 2. hearing detection triggers alert
local function test_hearing_detection_triggers_alert()
    make_open_plane()
    local enemy = spawn_entity()
    set_ai_state(enemy, "idle")
    set_position(enemy, 1, 0, 0)
    set_hearing(enemy, 5, 1.0)

    local emitter = spawn_entity()
    set_position(emitter, 0, 0, 0)
    emit_noise(emitter, 0.5, 5)

    tick()

    local ai = get_ai_state(enemy)
    assert.not_nil(ai, "Enemy should have EnemyAI")
    assert.is_true(ai.alert_level > 0, "alert_level should increase after hearing noise")
    assert.equals(ai.state, "investigate", "Enemy should investigate after hearing noise")
end

-- 3. stealth reduces noise
local function test_stealth_reduces_noise()
    make_open_plane()
    local eid = spawn_entity()
    set_position(eid, 0, 0, 0)
    emit_noise(eid, 1.0, 5)
    set_component(eid, "Stealth", { noise_modifier = 0.5 })

    tick()

    assert.equals(get_noise_at(0, 0, 0), 0.5, "Stealth should halve origin noise")
    assert.equals(get_noise_at(1, 0, 0), 0.4, "Stealth should halve propagated noise")
end

return {
    test_emit_noise_get_noise_at = test_emit_noise_get_noise_at,
    test_hearing_detection_triggers_alert = test_hearing_detection_triggers_alert,
    test_stealth_reduces_noise = test_stealth_reduces_noise,
}