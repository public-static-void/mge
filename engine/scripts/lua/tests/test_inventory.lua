local assert = require("assert")
local utils = require("utils")

local function test_inventory_crud()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), max_slots = 5, weight = 0.0, volume = 0.0 })
	local inv = get_inventory(e)
	assert.equals(inv.max_slots, 5)
	assert.equals(type(inv.slots), "table")
	assert.equals(#inv.slots, 0)
	assert.equals(inv.weight, 0.0)
	assert.equals(inv.volume, 0.0)
end

local function test_add_and_remove_item()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })

	local item_id = "sword"
	local item_entity = spawn_entity()
	set_component(item_entity, "Item", { id = item_id, name = "Sword", slot = "right_hand" })

	add_item_to_inventory(e, item_id)
	local inv = get_inventory(e)
	assert.equals(type(inv.slots), "table")
	assert.equals(inv.slots[1], item_id)

	remove_item_from_inventory(e, 0)
	inv = get_inventory(e)
	assert.equals(type(inv.slots), "table")
	assert.is_nil(inv.slots[1])
	assert.equals(#inv.slots, 0)
end

local function test_remove_item_out_of_bounds()
	local e = spawn_entity()
	set_inventory(e, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local ok, err = pcall(function()
		remove_item_from_inventory(e, 0)
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("out of bounds"), "Error text not found!")
end

return {
	test_inventory_crud = test_inventory_crud,
	test_add_and_remove_item = test_add_and_remove_item,
	test_remove_item_out_of_bounds = test_remove_item_out_of_bounds,
}
