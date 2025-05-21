local lu = require("luaunit")

function test_equip_and_unequip()
	print("START test_equip_and_unequip")
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	add_item_to_inventory(e, "sword")

	-- Equip the sword
	local ok, err = pcall(function()
		equip_item(e, "sword", "right_hand")
	end)
	assert(ok, "equip_item failed: " .. tostring(err))

	local eq = get_equipment(e)
	lu.assertEquals(eq.slots.right_hand, "sword")

	-- Unequip the sword
	ok, err = pcall(function()
		unequip_item(e, "right_hand")
	end)
	assert(ok, "unequip_item failed: " .. tostring(err))

	eq = get_equipment(e)
	lu.assertNil(eq.slots.right_hand)
end

function test_equip_invalid_slot()
	print("START test_equip_invalid_slot")
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	add_item_to_inventory(e, "sword")

	-- Try to equip to an invalid slot
	local ok, err = pcall(function()
		equip_item(e, "sword", "left_foot")
	end)
	assert(not ok, "Expected error, got success")
	assert(tostring(err):find("invalid slot"), "Error message mismatch: " .. tostring(err))
end

function test_equip_item_not_in_inventory()
	print("START test_equip_item_not_in_inventory")
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })

	-- Try to equip an item not in inventory
	local ok, err = pcall(function()
		equip_item(e, "sword", "right_hand")
	end)
	assert(not ok, "Expected error, got success")
	assert(tostring(err):find("not in inventory"), "Error message mismatch: " .. tostring(err))
end

function test_double_equip_same_slot()
	print("START test_double_equip_same_slot")
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local sword = spawn_entity()
	set_component(sword, "Item", { id = "sword", name = "Sword", slot = "right_hand" })
	local shield = spawn_entity()
	set_component(shield, "Item", { id = "shield", name = "Shield", slot = "right_hand" })
	add_item_to_inventory(e, "sword")
	add_item_to_inventory(e, "shield")

	local ok, err = pcall(function()
		equip_item(e, "sword", "right_hand")
	end)
	assert(ok, "equip_item failed: " .. tostring(err))

	ok, err = pcall(function()
		equip_item(e, "shield", "right_hand")
	end)
	assert(not ok, "Expected error, got success")
	assert(tostring(err):find("already equipped"), "Error message mismatch: " .. tostring(err))
end

os.exit(lu.LuaUnit.run())
