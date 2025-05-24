local assert = require("assert")

-- Helper to mark an empty Lua table as an array for Rust glue
local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

-- Recursively fix all ECS array fields to be empty arrays if they are empty tables
local function fix_arrays(obj)
	if type(obj) ~= "table" then
		return
	end

	-- Fix equipped
	if obj.equipped ~= nil then
		if type(obj.equipped) == "table" then
			if next(obj.equipped) == nil or (getmetatable(obj.equipped) and getmetatable(obj.equipped).__is_array) then
				obj.equipped = empty_array()
			else
				fix_arrays(obj.equipped)
			end
		end
	end

	-- Fix children
	if obj.children ~= nil then
		if type(obj.children) == "table" then
			if next(obj.children) == nil or (getmetatable(obj.children) and getmetatable(obj.children).__is_array) then
				obj.children = empty_array()
			else
				for _, child in ipairs(obj.children) do
					fix_arrays(child)
				end
			end
		end
	end

	-- Fix parts (top-level array of parts)
	if obj.parts ~= nil then
		if type(obj.parts) == "table" then
			if next(obj.parts) == nil or (getmetatable(obj.parts) and getmetatable(obj.parts).__is_array) then
				obj.parts = empty_array()
			else
				for _, part in ipairs(obj.parts) do
					fix_arrays(part)
				end
			end
		end
	end
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
	local right_hand_equipped = body.parts[1].children[1].children[1].equipped
	assert.equals(right_hand_equipped[1], "gold_ring")

	body.parts[1].children[1].children[1].status = "wounded"
	fix_arrays(body)
	set_component(e, "Body", body)
	run_native_system("BodyEquipmentSyncSystem")

	local body_after = get_component(e, "Body")
	local eq_after = get_equipment(e)
	assert.equals(#body_after.parts[1].children[1].children[1].equipped, 0)
	assert.not_nil(eq_after.slots)
	assert.is_nil(eq_after.slots.right_hand)
end

return {
	test_equipment_body_sync = test_equipment_body_sync,
}
