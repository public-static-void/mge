-- test_diplomacy.lua: Tests for faction-pair diplomacy (relations, treaties, war/peace).
-- Each test gets a fresh world via the test runner.
-- Global functions: get_relation, get_standing, modify_standing, declare_war,
-- declare_peace, propose_treaty, accept_treaty, break_treaty, list_treaties

local assert = require("assert")

-- 1. Unknown pairs start neutral with zero standing
local function test_unknown_pair_defaults()
    assert.equals(get_relation("athens", "sparta"), "neutral", "Unknown pair should be neutral")
    assert.equals(get_standing("athens", "sparta"), 0, "Unknown pair should have 0 standing")
end

-- 2. Standing mutation round-trips and is order-independent
local function test_modify_standing_roundtrip()
    modify_standing("athens", "sparta", 30)
    assert.equals(get_standing("athens", "sparta"), 30, "Standing should be 30")
    assert.equals(get_standing("sparta", "athens"), 30, "Pair lookup should ignore order")
end

-- 3. Standing clamps at bounds through the bridge
local function test_standing_clamps_at_bounds()
    modify_standing("athens", "sparta", 200)
    assert.equals(get_standing("athens", "sparta"), 100, "Standing should clamp to 100")
    modify_standing("athens", "sparta", -350)
    assert.equals(get_standing("athens", "sparta"), -100, "Standing should clamp to -100")
end

-- 4. Alliance proposal then acceptance raises the pair to allied
local function test_alliance_accept_sets_allied()
    local tid = propose_treaty("athens", "sparta", "alliance", nil)
    accept_treaty(tid)
    assert.equals(get_relation("athens", "sparta"), "allied", "Alliance should set allied")
end

-- 5. Full arc: propose, accept, war, peace ends neutral at zero
local function test_war_then_peace_resets_pair()
    local tid = propose_treaty("athens", "sparta", "alliance", nil)
    accept_treaty(tid)
    declare_war("athens", "sparta")
    assert.equals(get_relation("athens", "sparta"), "war", "War should set war state")
    declare_peace("athens", "sparta")
    assert.equals(get_relation("athens", "sparta"), "neutral", "Peace should reset to neutral")
    assert.equals(get_standing("athens", "sparta"), 0, "Peace should reset standing to 0")
end

-- 6. Accepting a peace treaty ends a war with the same end state
local function test_peace_treaty_accept_ends_war()
    declare_war("athens", "corinth")
    local tid = propose_treaty("athens", "corinth", "peace", nil)
    accept_treaty(tid)
    assert.equals(get_relation("athens", "corinth"), "neutral", "Peace treaty should end war")
    assert.equals(get_standing("athens", "corinth"), 0, "Peace treaty should reset standing")
end

-- 7. Non-peace proposals are rejected while the pair is at war
local function test_war_blocks_non_peace_proposal()
    declare_war("athens", "sparta")
    local ok, err = pcall(propose_treaty, "athens", "sparta", "non_aggression", nil)
    assert.is_false(ok, "Non-peace proposal during war should error")
    assert.is_true(string.find(tostring(err) or "", "at war") ~= nil, "Error should mention war")
end

-- 8. Self-pair proposals are rejected
local function test_self_pair_proposal_errors()
    local ok, err = pcall(propose_treaty, "athens", "athens", "alliance", nil)
    assert.is_false(ok, "Self-pair proposal should error")
    assert.is_true(string.find(tostring(err) or "", "self") ~= nil, "Error should mention self")
end

-- 9. Duplicate live proposals are rejected
local function test_duplicate_proposal_errors()
    propose_treaty("athens", "sparta", "alliance", nil)
    local ok, err = pcall(propose_treaty, "athens", "sparta", "alliance", nil)
    assert.is_false(ok, "Duplicate proposal should error")
    assert.is_true(string.find(tostring(err) or "", "duplicate") ~= nil, "Error should mention duplicate")
end

-- 10. Breaking an active treaty applies the standing penalty
local function test_break_applies_penalty()
    local tid = propose_treaty("athens", "sparta", "alliance", nil)
    accept_treaty(tid)
    break_treaty(tid)
    assert.equals(get_standing("athens", "sparta"), -25, "Break should apply -25 penalty")
end

-- 11. Treaty listing reports records with canonical kind and status names
local function test_list_treaties_reports_records()
    local first = propose_treaty("athens", "sparta", "alliance", nil)
    accept_treaty(first)
    propose_treaty("athens", "corinth", "trade", nil)
    local all = list_treaties(nil)
    assert.is_table(all, "list_treaties should return a table")
    assert.equals(#all, 2, "Two treaties should be listed")
    assert.equals(all[1].kind, "alliance", "First treaty kind should be alliance")
    assert.equals(all[1].status, "active", "Accepted treaty should be active")
    assert.equals(all[2].kind, "trade", "Second treaty kind should be trade")
    local filtered = list_treaties("corinth")
    assert.equals(#filtered, 1, "Filter should return only corinth treaties")
    assert.equals(filtered[1].kind, "trade", "Filtered treaty should be the trade treaty")
end

return {
    test_unknown_pair_defaults = test_unknown_pair_defaults,
    test_modify_standing_roundtrip = test_modify_standing_roundtrip,
    test_standing_clamps_at_bounds = test_standing_clamps_at_bounds,
    test_alliance_accept_sets_allied = test_alliance_accept_sets_allied,
    test_war_then_peace_resets_pair = test_war_then_peace_resets_pair,
    test_peace_treaty_accept_ends_war = test_peace_treaty_accept_ends_war,
    test_war_blocks_non_peace_proposal = test_war_blocks_non_peace_proposal,
    test_self_pair_proposal_errors = test_self_pair_proposal_errors,
    test_duplicate_proposal_errors = test_duplicate_proposal_errors,
    test_break_applies_penalty = test_break_applies_penalty,
    test_list_treaties_reports_records = test_list_treaties_reports_records,
}
