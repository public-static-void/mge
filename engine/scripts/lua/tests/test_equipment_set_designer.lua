-- test_equipment_set_designer.lua: Tests for equipment set designer APIs.
-- Functions: define_equipment_set, apply_loadout, get_loadout

local assert = require("assert")
local utils = require("utils")

local function setup_items()
	register_item('{"id":"test_weapon","name":"Test Weapon","slot":"weapon"}')
	register_item('{"id":"test_shield","name":"Test Shield","slot":"shield"}')
	register_item('{"id":"test_helmet","name":"Test Helmet","slot":"helmet"}')
end

local function setup_entity_with_inventory()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	return e
end

-- define_equipment_set: registers set successfully
local function test_define_equipment_set()
	setup_items()
	define_equipment_set("warrior_loadout", { weapon = "test_weapon", shield = "test_shield" })
	-- Verify by applying it to an entity
	local e = setup_entity_with_inventory()
	local result = apply_loadout(e, "warrior_loadout")
	assert.equals(result, e, "apply_loadout should return entity ID")
end

-- define_equipment_set: overwrite same name
local function test_define_equipment_set_overwrite()
	setup_items()
	define_equipment_set("overwrite_set", { weapon = "test_weapon" })
	define_equipment_set("overwrite_set", { weapon = "test_weapon", shield = "test_shield" })
	local e = setup_entity_with_inventory()
	local result = apply_loadout(e, "overwrite_set")
	assert.equals(result, e)
	local eq = get_equipment(e)
	assert.equals(eq.slots.weapon, "test_weapon")
	assert.equals(eq.slots.shield, "test_shield")
end

-- apply_loadout: fills equipment slots correctly
local function test_apply_loadout_fills_slots()
	setup_items()
	define_equipment_set("full_loadout", {
		weapon = "test_weapon",
		shield = "test_shield",
		helmet = "test_helmet",
	})
	local e = setup_entity_with_inventory()
	apply_loadout(e, "full_loadout")
	local eq = get_equipment(e)
	assert.equals(eq.slots.weapon, "test_weapon")
	assert.equals(eq.slots.shield, "test_shield")
	assert.equals(eq.slots.helmet, "test_helmet")
end

-- apply_loadout: adds items to inventory
local function test_apply_loadout_adds_to_inventory()
	setup_items()
	define_equipment_set("inv_loadout", { weapon = "test_weapon" })
	local e = setup_entity_with_inventory()
	apply_loadout(e, "inv_loadout")
	local inv = get_inventory(e)
	assert.is_true(#inv.slots >= 1, "Inventory should contain at least one item")
end

-- apply_loadout: error on nonexistent set
local function test_apply_loadout_nonexistent_set()
	local e = setup_entity_with_inventory()
	local ok, err = pcall(function()
		apply_loadout(e, "no_such_set")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("not found"), "Error should mention set not found")
end

-- apply_loadout: error on entity without Inventory
local function test_apply_loadout_no_inventory()
	setup_items()
	define_equipment_set("inv_test", { weapon = "test_weapon" })
	local e = spawn_entity()
	local ok, err = pcall(function()
		apply_loadout(e, "inv_test")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("Inventory"), "Error should mention Inventory")
end

-- apply_loadout: error when item not registered
local function test_apply_loadout_unregistered_item()
	define_equipment_set("bad_item_set", { weapon = "ghost_item" })
	local e = setup_entity_with_inventory()
	local ok, err = pcall(function()
		apply_loadout(e, "bad_item_set")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("not found") or msg.msg:find("registry"), "Error should mention unregistered item")
end

-- get_loadout: returns matching set when equipped
local function test_get_loadout_matching()
	setup_items()
	define_equipment_set("match_set", { weapon = "test_weapon", shield = "test_shield" })
	local e = setup_entity_with_inventory()
	apply_loadout(e, "match_set")
	local result = get_loadout(e)
	assert.not_nil(result, "get_loadout should find matching set")
	assert.equals(result.name, "match_set")
	assert.equals(result.items.weapon, "test_weapon")
	assert.equals(result.items.shield, "test_shield")
end

-- get_loadout: returns nil when no match
local function test_get_loadout_no_match()
	setup_items()
	define_equipment_set("no_match_set", { weapon = "test_weapon" })
	local e = setup_entity_with_inventory()
	local result = get_loadout(e)
	assert.is_nil(result, "get_loadout should return nil for empty equipment")
end

-- get_loadout: returns nil for entity without Equipment
local function test_get_loadout_no_equipment()
	local e = spawn_entity()
	local result = get_loadout(e)
	assert.is_nil(result, "get_loadout should return nil for entity without Equipment")
end

-- get_loadout: partial match returns nil (exact match required)
local function test_get_loadout_partial_no_match()
	setup_items()
	define_equipment_set("partial_set", { weapon = "test_weapon", shield = "test_shield" })
	local e = setup_entity_with_inventory()
	-- Equip only weapon
	set_component(e, "Equipment", { slots = { weapon = "test_weapon" } })
	local result = get_loadout(e)
	assert.is_nil(result, "get_loadout should not match partial equipment")
end

-- load_equipment_sets: errors on nonexistent directory
local function test_load_equipment_sets_bad_dir()
	local ok, err = pcall(function()
		load_equipment_sets("/nonexistent/path/to/sets")
	end)
	assert.is_false(ok, "Loading from nonexistent directory should error")
end

return {
	test_define_equipment_set = test_define_equipment_set,
	test_define_equipment_set_overwrite = test_define_equipment_set_overwrite,
	test_apply_loadout_fills_slots = test_apply_loadout_fills_slots,
	test_apply_loadout_adds_to_inventory = test_apply_loadout_adds_to_inventory,
	test_apply_loadout_nonexistent_set = test_apply_loadout_nonexistent_set,
	test_apply_loadout_no_inventory = test_apply_loadout_no_inventory,
	test_apply_loadout_unregistered_item = test_apply_loadout_unregistered_item,
	test_get_loadout_matching = test_get_loadout_matching,
	test_get_loadout_no_match = test_get_loadout_no_match,
	test_get_loadout_no_equipment = test_get_loadout_no_equipment,
	test_get_loadout_partial_no_match = test_get_loadout_partial_no_match,
	test_load_equipment_sets_bad_dir = test_load_equipment_sets_bad_dir,
}
