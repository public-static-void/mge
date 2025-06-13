local assert = require("assert")
local utils = require("utils")

local function test_equip_and_unequip()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	add_item_to_inventory(e, "sword")

	equip_item(e, "sword", "right_hand")

	local eq = get_equipment(e)
	assert.equals(eq.slots.right_hand, "sword")

	unequip_item(e, "right_hand")
	eq = get_equipment(e)
	assert.is_nil(eq.slots.right_hand)
end

local function test_equip_invalid_slot()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	add_item_to_inventory(e, "sword")
	local ok, err = pcall(function()
		equip_item(e, "sword", "left_foot")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("Invalid slot"), "Error text not found!")
end

local function test_equip_item_not_in_inventory()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	local ok, err = pcall(function()
		equip_item(e, "sword", "right_hand")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("not in Inventory"), "Error text not found!")
end

local function test_double_equip_same_slot()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	local shield = spawn_entity()
	set_component(shield, "Item", { id = "shield", name = "Shield", slot = "right_hand" })
	add_item_to_inventory(e, "sword")
	add_item_to_inventory(e, "shield")

	equip_item(e, "sword", "right_hand")
	local ok, err = pcall(function()
		equip_item(e, "shield", "right_hand")
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("already occupied"), "Error text not found!")
end

return {
	test_equip_and_unequip = test_equip_and_unequip,
	test_equip_invalid_slot = test_equip_invalid_slot,
	test_equip_item_not_in_inventory = test_equip_item_not_in_inventory,
	test_double_equip_same_slot = test_double_equip_same_slot,
}
