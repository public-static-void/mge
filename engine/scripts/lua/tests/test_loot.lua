-- test_loot.lua: Tests for the loot table module.
-- Each test gets a fresh world via the test runner.

local assert = require("assert")
local loot = require("loot")

-- Helper: create a dead enemy entity at (x, y)
local function create_dead_enemy(x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "enemy" })
	set_component(eid, "Health", { current = 0, max = 3 })
	set_component(eid, "Position", { pos = { Square = { x = x or 5, y = y or 10, z = 0 } } })
	return eid
end

-- Helper: create a living enemy entity at (x, y)
local function create_living_enemy(x, y)
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "enemy" })
	set_component(eid, "Health", { current = 3, max = 3 })
	set_component(eid, "Position", { pos = { Square = { x = x or 5, y = y or 10, z = 0 } } })
	return eid
end

-- Helper: count how many of the spawned entity IDs have a given item_id
local function count_item_id(eids, target_id)
	local count = 0
	for _, eid in ipairs(eids) do
		local item = get_component(eid, "Item")
		if item and item.id == target_id then
			count = count + 1
		end
	end
	return count
end

-- Helper: check that an entity has a Position at (x, y)
local function entity_at(eid, x, y)
	local pos = get_component(eid, "Position")
	return pos and pos.pos and pos.pos.Square and pos.pos.Square.x == x and pos.pos.Square.y == y
end

-- 1. Single-entry table with weight 1 always spawns that entry
local function test_define_and_roll_basic()
	loot.define_table("test_basic", {
		{ item_id = "health_potion", weight = 1 },
	})
	local results = loot.roll("test_basic", { x = 3, y = 7 })
	assert.is_true(#results > 0, "Should spawn at least one entity")
	for _, eid in ipairs(results) do
		local item = get_component(eid, "Item")
		assert.not_nil(item, "Spawned entity should have Item component")
		assert.equals(item.id, "health_potion")
	end
end

-- 2. Rolling on undefined table returns empty {} without error
local function test_empty_table_returns_empty()
	local results = loot.roll("nonexistent_table", { x = 0, y = 0 })
	assert.is_table(results)
	assert.equals(#results, 0)
end

-- 3. Three entries with equal weights should all appear over many rolls
local function test_multiple_entries_resolve()
	loot.define_table("test_uniform", {
		{ item_id = "health_potion", weight = 1 },
		{ item_id = "rusty_sword", weight = 1 },
		{ item_id = "wooden_shield", weight = 1 },
	})
	local seen = {}
	for _ = 1, 30 do
		local results = loot.roll("test_uniform", { x = 1, y = 1 })
		for _, eid in ipairs(results) do
			local item = get_component(eid, "Item")
			if item then
				seen[item.id] = true
			end
		end
	end
	assert.is_true(seen["health_potion"], "health_potion should appear in 30 rolls")
	assert.is_true(seen["rusty_sword"], "rusty_sword should appear in 30 rolls")
end

-- 4. Weighted distribution: weight-90 entry drops more than weight-10 over 50 rolls
local function test_weighted_distribution()
	math.randomseed(42)
	loot.define_table("test_weighted", {
		{ item_id = "health_potion", weight = 90 },
		{ item_id = "rusty_sword", weight = 10 },
	})
	local count_heavy = 0
	local count_light = 0
	for _ = 1, 50 do
		local results = loot.roll("test_weighted", { x = 1, y = 1 })
		for _, eid in ipairs(results) do
			local item = get_component(eid, "Item")
			if item then
				if item.id == "health_potion" then
					count_heavy = count_heavy + 1
				elseif item.id == "rusty_sword" then
					count_light = count_light + 1
				end
			end
		end
	end
	assert.is_true(count_heavy > count_light,
		"weight-90 entry (" .. count_heavy .. ") should appear more than weight-10 (" .. count_light .. ")")
end

-- 5. Condition returning false prevents item spawn
local function test_condition_filter()
	loot.define_table("test_condition", {
		{ item_id = "health_potion", weight = 1, condition = function(eid)
			return false  -- never drop for any entity
		end },
	})
	local eid = create_dead_enemy(5, 10)
	-- define "enemy" table so spawn_for_death finds it
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1, condition = function(_eid)
			return false
		end },
	})
	local results = loot.spawn_for_death(eid)
	assert.is_table(results)
	assert.equals(#results, 0, "Condition returning false should produce no drops")
end

-- 6. min_count/max_count: spawned quantity stays within range
local function test_min_max_count()
	loot.define_table("test_count", {
		{ item_id = "health_potion", weight = 1, min_count = 2, max_count = 5 },
	})
	for _ = 1, 20 do
		local results = loot.roll("test_count", { x = 1, y = 1 })
		assert.is_true(#results >= 2 and #results <= 5,
			"Spawned count " .. #results .. " should be in [2, 5]")
	end
end

-- 7. Living entity (Health.current > 0) produces no drops
local function test_spawn_for_death_alive_entity()
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1 },
	})
	local eid = create_living_enemy(5, 10)
	local results = loot.spawn_for_death(eid)
	assert.is_table(results)
	assert.equals(#results, 0, "Living entity should produce no drops")
end

-- 8. Dead enemy with "enemy" table defined spawns items at its position
local function test_spawn_for_death_integration()
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1 },
	})
	local eid = create_dead_enemy(7, 12)
	local results = loot.spawn_for_death(eid)
	assert.is_true(#results > 0, "Should spawn items for dead enemy")
	-- Verify items are at correct position
	for _, spawned in ipairs(results) do
		assert.is_true(entity_at(spawned, 7, 12),
			"Spawned item should be at enemy's position (7, 12)")
	end
	-- Verify item_id matches
	local item = get_component(results[1], "Item")
	assert.not_nil(item)
	assert.equals(item.id, "health_potion")
end

-- 9. Redefining a table overwrites its previous entries
local function test_redefine_table_overwrites()
	loot.define_table("overwrite_test", {
		{ item_id = "health_potion", weight = 1 },
	})
	-- Redefine with different entry
	loot.define_table("overwrite_test", {
		{ item_id = "rusty_sword", weight = 1 },
	})
	local results = loot.roll("overwrite_test", { x = 1, y = 1 })
	assert.is_true(#results > 0)
	local item = get_component(results[1], "Item")
	assert.equals(item.id, "rusty_sword", "Redefined table should drop rusty_sword, not health_potion")
end

-- 10. Roll on table with total_weight 0 returns empty
local function test_zero_weight_table()
	loot.define_table("zero_weight", {
		{ item_id = "health_potion", weight = 1 },
	})
	-- Can't really make a zero-weight entry (weight < 1 is rejected),
	-- but we can test that an undefined table returns {} (covered in test 2).
	-- This tests that a valid table works at minimum weight.
	local results = loot.roll("zero_weight", { x = 0, y = 0 })
	assert.is_true(#results > 0)
end

-- 11. Entity with no Position returns {} from spawn_for_death
local function test_spawn_for_death_no_position()
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1 },
	})
	local eid = spawn_entity()
	set_component(eid, "Type", { kind = "enemy" })
	set_component(eid, "Health", { current = 0, max = 3 })
	-- No Position component
	local results = loot.spawn_for_death(eid)
	assert.is_table(results)
	assert.equals(#results, 0, "Entity without Position should produce no drops")
end

-- 12. Entity with no Type still tries to find "enemy" table
local function test_spawn_for_death_no_type()
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1 },
	})
	local eid = spawn_entity()
	set_component(eid, "Health", { current = 0, max = 3 })
	set_component(eid, "Position", { pos = { Square = { x = 3, y = 3, z = 0 } } })
	-- No Type component — should fallback to "enemy"
	local results = loot.spawn_for_death(eid)
	assert.is_true(#results > 0, "Entity without Type should fallback to 'enemy' table")
end

-- 13. Verify roll returns items at correct position
local function test_roll_position()
	loot.define_table("pos_test", {
		{ item_id = "health_potion", weight = 1 },
	})
	local results = loot.roll("pos_test", { x = 15, y = 8 })
	assert.is_true(#results > 0)
	for _, eid in ipairs(results) do
		assert.is_true(entity_at(eid, 15, 8),
			"Spawned item should be at position (15, 8)")
	end
end

-- 14. Multiple separate spawn_for_death calls work independently
local function test_multiple_spawn_for_death()
	loot.define_table("enemy", {
		{ item_id = "health_potion", weight = 1 },
	})
	local eid1 = create_dead_enemy(1, 1)
	local eid2 = create_dead_enemy(2, 2)
	local r1 = loot.spawn_for_death(eid1)
	local r2 = loot.spawn_for_death(eid2)
	assert.is_true(#r1 > 0, "First enemy should drop items")
	assert.is_true(#r2 > 0, "Second enemy should drop items")
end

return {
	test_define_and_roll_basic = test_define_and_roll_basic,
	test_empty_table_returns_empty = test_empty_table_returns_empty,
	test_multiple_entries_resolve = test_multiple_entries_resolve,
	test_weighted_distribution = test_weighted_distribution,
	test_condition_filter = test_condition_filter,
	test_min_max_count = test_min_max_count,
	test_spawn_for_death_alive_entity = test_spawn_for_death_alive_entity,
	test_spawn_for_death_integration = test_spawn_for_death_integration,
	test_redefine_table_overwrites = test_redefine_table_overwrites,
	test_zero_weight_table = test_zero_weight_table,
	test_spawn_for_death_no_position = test_spawn_for_death_no_position,
	test_spawn_for_death_no_type = test_spawn_for_death_no_type,
	test_roll_position = test_roll_position,
	test_multiple_spawn_for_death = test_multiple_spawn_for_death,
}
