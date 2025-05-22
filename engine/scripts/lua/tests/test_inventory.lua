local lu = require("luaunit")

function test_inventory_crud()
	local ok, err = pcall(function()
		local e = spawn_entity()
		set_inventory(e, { slots = {}, max_slots = 5, weight = 0.0, volume = 0.0 })
		local inv = get_inventory(e)
		lu.assertEquals(inv.max_slots, 5)
		lu.assertEquals(#inv.slots, 0)
		lu.assertEquals(inv.weight, 0.0)
		lu.assertEquals(inv.volume, 0.0)
	end)
	assert(ok, "Unexpected error in CRUD: " .. tostring(err))
end

function test_add_and_remove_item()
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })

	local item_id = "sword"
	local item_entity = spawn_entity()
	set_component(item_entity, "Item", { id = item_id, name = "Sword", slot = "right_hand" })

	add_item_to_inventory(e, item_id)
	local inv = get_inventory(e)
	lu.assertEquals(inv.slots[1], item_id)

	-- Use 0-based index!
	remove_item_from_inventory(e, 0)
	inv = get_inventory(e)
	lu.assertEquals(#inv.slots, 0)
end

function test_remove_item_out_of_bounds()
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local ok, err = pcall(function()
		remove_item_from_inventory(e, 0)
	end)
	assert(not ok, "Expected error, got success")
	assert(tostring(err):find("out of bounds"), "Error message mismatch: " .. tostring(err))
end

os.exit(lu.LuaUnit.run())
