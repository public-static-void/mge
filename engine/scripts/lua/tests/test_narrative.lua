-- test_narrative.lua: Tests for the event-driven narrative engine (incident director).
-- Each test gets a fresh world via the test runner.
-- Global functions: register_scenario, list_scenarios, get_scenario,
-- poll_pending_decisions, get_pending_decision, resolve_decision,
-- get_narrative_history. Trigger evaluation stays in Rust core; the bridge
-- only registers, polls, and resolves.

local assert = require("assert")

local function scenario_def(id, triggers, choices)
    return '{"id":"' .. id .. '","name":"Scenario ' .. id .. '","triggers":' .. triggers .. ',"choices":' .. choices .. '}'
end

-- Host ticks evaluate systems before the turn increment, so turn 0 fires on
-- the first tick on every bridge (WASM ticks increment first and agree).
local function turn_gate_def(id)
    return scenario_def(
        id,
        '[{"type":"turn_gte","turn":0}]',
        '[{"id":"take","label":"Take","effects":[{"action":"emit_event","data":{"bus":"narr_test_bus","payload":{"note":"picked"}}}]}]'
    )
end

-- 1. Registration round-trips through list and get
local function test_register_list_get_roundtrip()
    register_scenario(turn_gate_def("lua_alpha"))
    local all = list_scenarios()
    assert.is_table(all, "list_scenarios should return a table")
    assert.equals(#all, 1, "One scenario should be listed")
    assert.equals(all[1].id, "lua_alpha", "Listed scenario should carry its id")
    local one = get_scenario("lua_alpha")
    assert.is_table(one, "get_scenario should return a table")
    assert.equals(one.name, "Scenario lua_alpha", "Definition name should round-trip")
    assert.is_nil(get_scenario("missing"), "Unknown id should return nil")
end

-- 2. Duplicate registration replaces without duplicating evaluation
local function test_duplicate_registration_replaces()
    register_scenario(turn_gate_def("lua_dup"))
    register_scenario(turn_gate_def("lua_dup"))
    assert.equals(#list_scenarios(), 1, "Duplicate id should not duplicate the listing")
    tick()
    local pending = poll_pending_decisions()
    assert.equals(#pending, 1, "Duplicate registration should fire exactly once")
end

-- 3. Malformed definitions are rejected with an error
local function test_malformed_registration_errors()
    local ok, err = pcall(register_scenario, '{"id":"","name":"bad","choices":[]}')
    assert.is_false(ok, "Empty id with no choices should error")
    assert.is_true(string.find(tostring(err) or "", "id") ~= nil, "Error should mention the id")
    assert.equals(#list_scenarios(), 0, "Rejected definition should leave state untouched")
end

-- 4. Tick firing surfaces exactly one pending decision
local function test_tick_fires_pending_decision()
    register_scenario(turn_gate_def("lua_fire"))
    assert.equals(#poll_pending_decisions(), 0, "No pending decision should exist before the tick")
    tick()
    local pending = poll_pending_decisions()
    assert.equals(#pending, 1, "One pending decision should exist after the tick")
    assert.equals(pending[1].scenario_id, "lua_fire", "Pending decision should name its scenario")
    tick()
    assert.equals(#poll_pending_decisions(), 1, "Held trigger should not refire while pending")
end

-- 5. Pending lookup returns the decision or nil
local function test_get_pending_decision_lookup()
    register_scenario(turn_gate_def("lua_lookup"))
    tick()
    local pending = poll_pending_decisions()
    local one = get_pending_decision(pending[1].id)
    assert.is_table(one, "get_pending_decision should return a table")
    assert.equals(one.scenario_id, "lua_lookup", "Decision should name its scenario")
    assert.is_nil(get_pending_decision(9999), "Unknown decision id should return nil")
end

-- 6. Resolve applies the choice effect and records history
local function test_resolve_applies_effect_and_history()
    register_scenario(turn_gate_def("lua_resolve"))
    tick()
    local pending = poll_pending_decisions()
    resolve_decision(pending[1].id, "take")
    assert.equals(#poll_pending_decisions(), 0, "Resolved decision should leave pending")
    -- Direct resolves bypass the tick buffer swap, so flush before polling.
    update_event_buses()
    local events = poll_ecs_event("narr_test_bus")
    assert.equals(#events, 1, "Choice emit_event effect should land on its bus")
    assert.equals(events[1].note, "picked", "Bus payload should carry the choice effect")
    local history = get_narrative_history()
    assert.equals(#history, 2, "Fire plus resolve should append two records")
    assert.equals(history[1].kind, "fired", "First record should be the firing")
    assert.equals(history[2].kind, "resolved", "Second record should be the resolution")
    assert.equals(history[2].choice_id, "take", "Resolved record should name the choice")
end

-- 7. Unknown decision and choice ids error with no state change
local function test_unknown_resolve_errors()
    register_scenario(turn_gate_def("lua_err"))
    tick()
    local ok, err = pcall(resolve_decision, 9999, "take")
    assert.is_false(ok, "Unknown decision id should error")
    assert.is_true(string.find(tostring(err) or "", "9999") ~= nil, "Error should name the decision")
    local pending = poll_pending_decisions()
    local bad_choice_ok = pcall(resolve_decision, pending[1].id, "ghost")
    assert.is_false(bad_choice_ok, "Unknown choice id should error")
    assert.equals(#poll_pending_decisions(), 1, "Failed resolve should keep pending")
    assert.equals(#get_narrative_history(), 1, "Failed resolve should append no record")
end

return {
    test_register_list_get_roundtrip = test_register_list_get_roundtrip,
    test_duplicate_registration_replaces = test_duplicate_registration_replaces,
    test_malformed_registration_errors = test_malformed_registration_errors,
    test_tick_fires_pending_decision = test_tick_fires_pending_decision,
    test_get_pending_decision_lookup = test_get_pending_decision_lookup,
    test_resolve_applies_effect_and_history = test_resolve_applies_effect_and_history,
    test_unknown_resolve_errors = test_unknown_resolve_errors,
}
