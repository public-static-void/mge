-- test_designer_validation.lua: Tests for validate_equipment designer API.
-- Function: validate_equipment

local assert = require("assert")
local utils = require("utils")

local function setup_items()
	register_item('{"id":"valid_sword","name":"Valid Sword","slot":"weapon"}')
	register_item('{"id":"valid_shield","name":"Valid Shield","slot":"shield"}')
end

-- validate_equipment: empty equipment returns no issues
local function test_validate_equipment_empty()
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = {} })
	local result = validate_equipment(e)
	assert.is_table(result, "validate_equipment should return a table")
	assert.is_table(result.issues, "Result should have issues array")
	assert.equals(#result.issues, 0, "Empty equipment should have no issues")
end

-- validate_equipment: entity without Equipment returns no issues
local function test_validate_equipment_no_equipment()
	local e = spawn_entity()
	local result = validate_equipment(e)
	assert.equals(#result.issues, 0, "Entity without Equipment should have no issues")
end

-- validate_equipment: unknown slot returns issue
local function test_validate_equipment_unknown_slot()
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = { bogus_slot = "some_item" } })
	local result = validate_equipment(e)
	assert.is_true(#result.issues > 0, "Unknown slot should produce issues")
	local found = false
	for _, issue in ipairs(result.issues) do
		if issue.reason == "unknown_slot" then
			found = true
			break
		end
	end
	assert.is_true(found, "Should report unknown_slot reason")
end

-- validate_equipment: unregistered item returns issue
local function test_validate_equipment_unregistered_item()
	setup_items()
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = { weapon = "ghost_item" } })
	local result = validate_equipment(e)
	assert.is_true(#result.issues > 0, "Unregistered item should produce issues")
	local found = false
	for _, issue in ipairs(result.issues) do
		if issue.reason == "item_not_registered" then
			found = true
			break
		end
	end
	assert.is_true(found, "Should report item_not_registered reason")
end

-- validate_equipment: valid equipment returns no issues
local function test_validate_equipment_valid()
	setup_items()
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = { weapon = "valid_sword" } })
	local result = validate_equipment(e)
	assert.equals(#result.issues, 0, "Valid equipment should have no issues")
end

-- validate_equipment: slot_mismatch returns issue
local function test_validate_equipment_slot_mismatch()
	setup_items()
	-- Shield item is registered for "shield" slot, but placed in "weapon" slot
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = { weapon = "valid_shield" } })
	local result = validate_equipment(e)
	assert.is_true(#result.issues > 0, "Slot mismatch should produce issues")
	local found = false
	for _, issue in ipairs(result.issues) do
		if issue.reason == "slot_mismatch" then
			found = true
			break
		end
	end
	assert.is_true(found, "Should report slot_mismatch reason")
end

-- validate_equipment: multiple issues reported
local function test_validate_equipment_multiple_issues()
	local e = spawn_entity()
	set_component(e, "Equipment", {
		slots = {
			bogus_slot = "item_a",
			weapon = "missing_item",
		},
	})
	local result = validate_equipment(e)
	assert.is_true(#result.issues >= 2, "Multiple bad slots should produce multiple issues")
end

-- validate_equipment: null slot value skipped (no issue)
local function test_validate_equipment_null_slot_skipped()
	local e = spawn_entity()
	set_component(e, "Equipment", { slots = { weapon = nil } })
	local result = validate_equipment(e)
	assert.equals(#result.issues, 0, "Null slot values should be skipped")
end

-- validate_equipment: unmet_stat_requirements returns issue
-- validate_equipment reads from the "Stats" component (not "BaseStats")
local function test_validate_equipment_unmet_requirements()
	setup_items()
	-- Register an item with stat requirements
	register_item('{"id":"req_sword","name":"Req Sword","slot":"weapon","requirements":{"strength":20}}')
	local e = spawn_entity()
	-- Entity has low strength — set Stats component (what validate_equipment reads)
	set_component(e, "Stats", { strength = 5 })
	set_component(e, "Equipment", { slots = { weapon = "req_sword" } })
	local result = validate_equipment(e)
	assert.is_true(#result.issues > 0, "Unmet requirements should produce issues")
	local found = false
	for _, issue in ipairs(result.issues) do
		if issue.reason == "unmet_requirements" then
			found = true
			break
		end
	end
	assert.is_true(found, "Should report unmet_requirements reason")
end

-- validate_equipment: met_stat_requirements no issue
local function test_validate_equipment_met_requirements()
	setup_items()
	register_item('{"id":"req_sword_ok","name":"Req Sword OK","slot":"weapon","requirements":{"strength":5}}')
	local e = spawn_entity()
	set_component(e, "Stats", { strength = 10 })
	set_component(e, "Equipment", { slots = { weapon = "req_sword_ok" } })
	local result = validate_equipment(e)
	local has_unmet = false
	for _, issue in ipairs(result.issues) do
		if issue.reason == "unmet_requirements" then
			has_unmet = true
			break
		end
	end
	assert.is_false(has_unmet, "Met requirements should not trigger unmet_requirements")
end

return {
	test_validate_equipment_empty = test_validate_equipment_empty,
	test_validate_equipment_no_equipment = test_validate_equipment_no_equipment,
	test_validate_equipment_unknown_slot = test_validate_equipment_unknown_slot,
	test_validate_equipment_unregistered_item = test_validate_equipment_unregistered_item,
	test_validate_equipment_valid = test_validate_equipment_valid,
	test_validate_equipment_slot_mismatch = test_validate_equipment_slot_mismatch,
	test_validate_equipment_multiple_issues = test_validate_equipment_multiple_issues,
	test_validate_equipment_null_slot_skipped = test_validate_equipment_null_slot_skipped,
	test_validate_equipment_unmet_requirements = test_validate_equipment_unmet_requirements,
	test_validate_equipment_met_requirements = test_validate_equipment_met_requirements,
}
