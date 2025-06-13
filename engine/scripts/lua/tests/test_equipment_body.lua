local assert = require("assert")

local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

local function test_equipment_body_sync()
	local e = spawn_entity()
	set_inventory(e, { slots = empty_array(), weight = 0.0, volume = 0.0 })
	set_component(e, "Equipment", { slots = { right_hand = nil } })
	set_component(e, "Body", {
		parts = {
			{
				name = "torso",
				status = "healthy",
				kind = "flesh",
				temperature = 37.0,
				ideal_temperature = 37.0,
				insulation = 1.0,
				heat_loss = 0.1,
				children = {
					{
						name = "right_arm",
						status = "healthy",
						kind = "flesh",
						temperature = 37.0,
						ideal_temperature = 37.0,
						insulation = 1.0,
						heat_loss = 0.1,
						children = {
							{
								name = "right_hand",
								status = "healthy",
								kind = "flesh",
								temperature = 37.0,
								ideal_temperature = 37.0,
								insulation = 1.0,
								heat_loss = 0.1,
								equipped = empty_array(),
								children = empty_array(),
							},
						},
						equipped = empty_array(),
					},
				},
				equipped = empty_array(),
			},
		},
	})

	local ring = spawn_entity()
	set_component(ring, "Item", { id = "gold_ring", name = "Gold Ring", slot = "right_hand" })
	add_item_to_inventory(e, "gold_ring")

	local ok2, err2 = pcall(function()
		equip_item(e, "gold_ring", "right_hand")
	end)
	assert.is_true(ok2, "equip_item failed: " .. tostring(err2))

	run_native_system("BodyEquipmentSyncSystem")

	local body = get_component(e, "Body")
	assert.equals(#body.parts, 1)
	assert.equals(#body.parts[1].children, 1)
	assert.equals(#body.parts[1].children[1].children, 1)
	local right_hand_equipped = body.parts[1].children[1].children[1].equipped
	assert.equals(right_hand_equipped[1], "gold_ring")

	body.parts[1].children[1].children[1].status = "wounded"
	set_component(e, "Body", body)
	run_native_system("BodyEquipmentSyncSystem")

	local body_after = get_component(e, "Body")
	local eq_after = get_equipment(e)
	assert.equals(#body_after.parts[1].children[1].children[1].equipped, 0)
	assert.not_nil(eq_after.slots)
	-- Defensive: only check .right_hand if slots is present and is a table
	if eq_after.slots and type(eq_after.slots) == "table" then
		assert.is_nil(eq_after.slots.right_hand)
	end
end

return {
	test_equipment_body_sync = test_equipment_body_sync,
}
