-- test_faction_reputation.lua: Tests for the faction and reputation system.
-- Each test gets a fresh world via the test runner.
-- Global functions: set_faction, get_faction, modify_reputation, get_reputation

local assert = require("assert")

-- 1. Set and get faction
local function test_set_and_get_faction()
    local id = spawn_entity()
    set_faction(id, "goblins", "member")
    local faction_id = get_faction(id)
    assert.equals(faction_id, "goblins", "Entity should have faction 'goblins'")
end

-- 2. Get faction returns nil when entity has no Faction component
local function test_get_faction_none()
    local id = spawn_entity()
    local faction_id = get_faction(id)
    assert.is_nil(faction_id, "Entity without Faction should return nil")
end

-- 3. Modify and get reputation
local function test_modify_and_get_reputation()
    local id = spawn_entity()
    modify_reputation(id, "goblins", 25)
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 25, "Reputation should be 25")
end

-- 4. Reputation clamping (upper bound)
local function test_reputation_clamp_positive()
    local id = spawn_entity()
    modify_reputation(id, "goblins", 200)
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 100, "Reputation should be clamped to 100")
end

-- 5. Reputation clamping (lower bound)
local function test_reputation_clamp_negative()
    local id = spawn_entity()
    modify_reputation(id, "goblins", -200)
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, -100, "Reputation should be clamped to -100")
end

-- 6. Reputation returns 0 when entity has no Reputation component
local function test_reputation_no_component()
    local id = spawn_entity()
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 0, "Entity without Reputation should return 0")
end

-- 7. Reputation returns 0 for unknown faction_id
local function test_reputation_unknown_faction()
    local id = spawn_entity()
    modify_reputation(id, "goblins", 25)
    local rep = get_reputation(id, "humans")
    assert.equals(rep, 0, "Unknown faction should return 0")
end

-- 8. Reputation modification works cumulatively
local function test_reputation_cumulative()
    local id = spawn_entity()
    modify_reputation(id, "goblins", 10)
    modify_reputation(id, "goblins", 20)
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 30, "Reputation should be 30 after cumulative modifications")
end

-- 9. Set faction with different roles
local function test_set_faction_with_role()
    local id = spawn_entity()
    set_faction(id, "humans", "leader")
    local faction_id = get_faction(id)
    assert.equals(faction_id, "humans", "Entity should have faction 'humans'")
end

-- 10. Reputation decays toward zero each tick with configured decay_rate
local function test_reputation_decay()
    local id = spawn_entity()
    set_faction(id, "goblins", "member")
    -- Set Reputation with decay_rate via set_component (modify_reputation always sets decay_rate to 0.0)
    set_component(id, "Reputation", { values = { goblins = 5 }, decay_rate = 1.0 })
    tick()
    tick()
    tick()
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 2, "Reputation should decay from 5 to 2 after 3 ticks with decay_rate 1.0")
end

-- 11. Reputation does NOT decay when decay_rate is 0.0
local function test_reputation_no_decay_zero_rate()
    local id = spawn_entity()
    set_faction(id, "goblins", "member")
    set_component(id, "Reputation", { values = { goblins = 5 }, decay_rate = 0.0 })
    tick()
    local rep = get_reputation(id, "goblins")
    assert.equals(rep, 5, "Reputation should NOT decay with decay_rate 0.0")
end

return {
    test_set_and_get_faction = test_set_and_get_faction,
    test_get_faction_none = test_get_faction_none,
    test_modify_and_get_reputation = test_modify_and_get_reputation,
    test_reputation_clamp_positive = test_reputation_clamp_positive,
    test_reputation_clamp_negative = test_reputation_clamp_negative,
    test_reputation_no_component = test_reputation_no_component,
    test_reputation_unknown_faction = test_reputation_unknown_faction,
    test_reputation_cumulative = test_reputation_cumulative,
    test_set_faction_with_role = test_set_faction_with_role,
    test_reputation_decay = test_reputation_decay,
    test_reputation_no_decay_zero_rate = test_reputation_no_decay_zero_rate,
}
