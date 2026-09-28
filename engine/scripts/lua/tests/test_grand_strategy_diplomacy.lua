-- test_grand_strategy_diplomacy.lua: Grand-strategy diplomacy scenario.
-- Three rival nations (valoria, drakmor, kesh) driven through the public
-- diplomacy API only: initial relations, goodwill standing, alliance treaty,
-- war on the third nation, peace, and treaty-list lifecycle.
-- Each test gets a fresh world via the test runner.
-- Global functions: get_relation, get_standing, modify_standing, declare_war,
-- declare_peace, propose_treaty, accept_treaty, list_treaties

local assert = require("assert")

-- 1. Three nations start neutral with zero standing on every pair
local function test_three_nations_start_neutral()
    assert.equals(get_relation("valoria", "drakmor"), "neutral", "valoria/drakmor should start neutral")
    assert.equals(get_relation("valoria", "kesh"), "neutral", "valoria/kesh should start neutral")
    assert.equals(get_relation("drakmor", "kesh"), "neutral", "drakmor/kesh should start neutral")
    assert.equals(get_standing("valoria", "drakmor"), 0, "valoria/drakmor should start at 0")
    assert.equals(get_standing("valoria", "kesh"), 0, "valoria/kesh should start at 0")
    assert.equals(get_standing("drakmor", "kesh"), 0, "drakmor/kesh should start at 0")
end

-- 2. A goodwill gesture lifts one pair while the third nation is untouched
local function test_goodwill_gesture_lifts_one_pair()
    modify_standing("valoria", "drakmor", 40)
    assert.equals(get_standing("valoria", "drakmor"), 40, "Gesture should lift standing to 40")
    assert.equals(get_standing("drakmor", "valoria"), 40, "Pair lookup should ignore order")
    assert.equals(get_standing("valoria", "kesh"), 0, "Third nation should stay at 0")
end

-- 3. An accepted alliance unites two nations while the third stays neutral
local function test_alliance_unites_two_nations()
    local tid = propose_treaty("valoria", "drakmor", "alliance", nil)
    accept_treaty(tid)
    assert.equals(get_relation("valoria", "drakmor"), "allied", "Alliance should set allied")
    assert.equals(get_relation("valoria", "kesh"), "neutral", "Third nation should stay neutral")
end

-- 4. War on the third nation reads war while the alliance holds
local function test_war_on_third_leaves_alliance_intact()
    local tid = propose_treaty("valoria", "drakmor", "alliance", nil)
    accept_treaty(tid)
    declare_war("valoria", "kesh")
    assert.equals(get_relation("valoria", "kesh"), "war", "War pair should read war")
    assert.equals(get_relation("kesh", "valoria"), "war", "War read should ignore order")
    assert.equals(get_relation("valoria", "drakmor"), "allied", "Alliance should survive war on another pair")
end

-- 5. War on the allied pair breaks paper, then peace ends the war
local function test_war_breaks_alliance_then_peace_ends_war()
    local tid = propose_treaty("valoria", "drakmor", "alliance", nil)
    accept_treaty(tid)
    declare_war("valoria", "drakmor")
    assert.equals(get_relation("valoria", "drakmor"), "war", "War should override the alliance")
    declare_peace("valoria", "drakmor")
    assert.equals(get_relation("valoria", "drakmor"), "neutral", "Peace should restore neutral")
    assert.equals(get_standing("valoria", "drakmor"), 0, "Peace should reset standing to 0")
end

-- 6. Full arc: treaty list reflects the alliance/war/peace lifecycle
local function test_treaty_list_reflects_lifecycle()
    local alliance = propose_treaty("valoria", "drakmor", "alliance", nil)
    accept_treaty(alliance)
    declare_war("valoria", "kesh")
    declare_peace("valoria", "kesh")
    assert.equals(get_relation("valoria", "kesh"), "neutral", "Peace should restore neutral")
    local all = list_treaties(nil)
    assert.is_table(all, "list_treaties should return a table")
    assert.equals(#all, 1, "Only the alliance should be recorded")
    assert.equals(all[1].kind, "alliance", "Recorded treaty should be the alliance")
    assert.equals(all[1].status, "active", "Alliance should still be active")
    local kesh_only = list_treaties("kesh")
    assert.equals(#kesh_only, 0, "War/peace alone should record no treaties")
    local valoria_only = list_treaties("valoria")
    assert.equals(#valoria_only, 1, "Valoria filter should return the alliance")
end

return {
    test_three_nations_start_neutral = test_three_nations_start_neutral,
    test_goodwill_gesture_lifts_one_pair = test_goodwill_gesture_lifts_one_pair,
    test_alliance_unites_two_nations = test_alliance_unites_two_nations,
    test_war_on_third_leaves_alliance_intact = test_war_on_third_leaves_alliance_intact,
    test_war_breaks_alliance_then_peace_ends_war = test_war_breaks_alliance_then_peace_ends_war,
    test_treaty_list_reflects_lifecycle = test_treaty_list_reflects_lifecycle,
}
