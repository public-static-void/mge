-- test_tech_tree.lua: Tests for the tech tree and research system.
-- Each test gets a fresh world via the test runner.
-- Global functions: research_tech, cancel_research, clear_research_queue,
--   can_research_tech, is_tech_completed, get_completed_techs,
--   get_research_queue, get_research_queue_progress,
--   get_tech_tree, get_tech_node, get_tech_progress

local assert = require("assert")

-- 1. Get tech tree returns table of nodes
local function test_get_tech_tree()
    local tree = get_tech_tree()
    assert.is_table(tree, "get_tech_tree() should return a table")
    assert.is_true(#tree > 0, "Tech tree should have nodes")
end

-- 2. Get specific tech node
local function test_get_tech_node()
    local node = get_tech_node("bronze_working")
    assert.not_nil(node, "bronze_working should exist")
    assert.equals(node.name, "Bronze Working", "Node name should match")
end

-- 3. Get nonexistent tech node returns nil
local function test_get_tech_node_missing()
    local node = get_tech_node("nonexistent_tech")
    assert.is_nil(node, "Nonexistent tech should return nil")
end

-- 4. Research tech adds to queue
local function test_research_tech()
    local id = spawn_entity()
    research_tech(id, "bronze_working")
    local queue = get_research_queue(id)
    assert.equals(#queue, 1, "Queue should have 1 entry")
    assert.equals(queue[1], "bronze_working", "Queue should contain bronze_working")
end

-- 5. Cannot research already completed tech
local function test_cannot_research_completed()
    local id = spawn_entity()
    -- Manually set TechProgress with bronze_working completed
    set_component(id, "TechProgress", {
        completed = { bronze_working = 1 },
        queue = {},
        queue_progress = {},
        research_points = 0,
    })
    local ok, err = pcall(research_tech, id, "bronze_working")
    assert.is_false(ok, "Should error when tech already completed")
    assert.is_true(string.find(tostring(err) or "", "already completed") ~= nil, "Error should mention already completed")
end

-- 6. Cannot research tech with unmet prerequisite
local function test_cannot_research_bad_prereq()
    local id = spawn_entity()
    -- iron_working requires bronze_working
    local ok, err = pcall(research_tech, id, "iron_working")
    assert.is_false(ok, "Should error when prerequisite not met")
    assert.is_true(string.find(tostring(err) or "", "Requires tech") ~= nil, "Error should mention requirement")
end

-- 7. Cannot research unknown tech
local function test_cannot_research_unknown()
    local id = spawn_entity()
    local ok, err = pcall(research_tech, id, "completely_unknown_tech")
    assert.is_false(ok, "Should error for unknown tech")
    assert.is_true(string.find(tostring(err) or "", "Unknown") ~= nil, "Error should mention Unknown")
end

-- 8. is_tech_completed returns correct state
local function test_is_tech_completed()
    local id = spawn_entity()
    -- Initially not completed
    assert.is_false(is_tech_completed(id, "bronze_working"), "Should not be completed initially")
    -- Mark as completed
    set_component(id, "TechProgress", {
        completed = { bronze_working = 1 },
        queue = {},
        queue_progress = {},
        research_points = 0,
    })
    assert.is_true(is_tech_completed(id, "bronze_working"), "Should be completed after marking")
end

-- 9. Cancel research removes from queue
local function test_cancel_research()
    local id = spawn_entity()
    research_tech(id, "bronze_working")
    assert.equals(#get_research_queue(id), 1, "Queue should have 1 entry")
    cancel_research(id, "bronze_working")
    assert.equals(#get_research_queue(id), 0, "Queue should be empty after cancel")
end

-- 10. Clear research queue empties queue
local function test_clear_research_queue()
    local id = spawn_entity()
    research_tech(id, "bronze_working")
    assert.equals(#get_research_queue(id), 1, "Queue should have 1 entry")
    clear_research_queue(id)
    assert.equals(#get_research_queue(id), 0, "Queue should be empty after clear")
end

-- 11. can_research_tech returns true for available tech
local function test_can_research_tech()
    local id = spawn_entity()
    local ok, reason = can_research_tech(id, "bronze_working")
    assert.is_true(ok, "bronze_working should be researchable")
    assert.equals(reason, "", "Reason should be empty for researchable tech")
end

-- 12. can_research_tech returns false for blocked tech
local function test_can_research_tech_blocked()
    local id = spawn_entity()
    local ok, reason = can_research_tech(id, "iron_working")
    assert.is_false(ok, "iron_working should not be researchable without bronze_working")
    assert.is_true(string.find(reason or "", "Requires tech") ~= nil, "Reason should mention requirement")
end

-- 13. get_completed_techs returns correctly
local function test_get_completed_techs()
    local id = spawn_entity()
    local completed = get_completed_techs(id)
    assert.is_table(completed, "Should return a table")
    assert.equals(#completed, 0, "Should be empty initially")
end

-- 14. get_research_queue_progress
local function test_get_research_queue_progress()
    local id = spawn_entity()
    research_tech(id, "bronze_working")
    local progress = get_research_queue_progress(id)
    assert.is_table(progress, "Should return a table")
    assert.equals(progress.bronze_working, 0, "Initial progress should be 0")
end

-- 15. Cannot research already queued tech
local function test_cannot_research_already_queued()
    local id = spawn_entity()
    research_tech(id, "bronze_working")
    local ok, err = pcall(research_tech, id, "bronze_working")
    assert.is_false(ok, "Should error when already in queue")
end

return {
    test_get_tech_tree = test_get_tech_tree,
    test_get_tech_node = test_get_tech_node,
    test_get_tech_node_missing = test_get_tech_node_missing,
    test_research_tech = test_research_tech,
    test_cannot_research_completed = test_cannot_research_completed,
    test_cannot_research_bad_prereq = test_cannot_research_bad_prereq,
    test_cannot_research_unknown = test_cannot_research_unknown,
    test_is_tech_completed = test_is_tech_completed,
    test_cancel_research = test_cancel_research,
    test_clear_research_queue = test_clear_research_queue,
    test_can_research_tech = test_can_research_tech,
    test_can_research_tech_blocked = test_can_research_tech_blocked,
    test_get_completed_techs = test_get_completed_techs,
    test_get_research_queue_progress = test_get_research_queue_progress,
    test_cannot_research_already_queued = test_cannot_research_already_queued,
}
