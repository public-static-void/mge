local assert = require("assert")

local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

local function make_humanoid_body()
	return {
		parts = {
			{
				name = "torso",
				kind = "torso",
				status = "healthy",
				hp = 50.0,
				max_hp = 50.0,
				temperature = 36.5,
				ideal_temperature = 36.5,
				insulation = 2.0,
				heat_loss = 0.1,
				children = {
					{
						name = "left arm",
						kind = "arm",
						status = "healthy",
						hp = 25.0,
						max_hp = 25.0,
						temperature = 35.0,
						ideal_temperature = 36.5,
						insulation = 1.0,
						heat_loss = 0.2,
						children = {
							{
								name = "left hand",
								kind = "hand",
								status = "healthy",
								hp = 10.0,
								max_hp = 10.0,
								temperature = 34.0,
								ideal_temperature = 36.5,
								insulation = 0.5,
								heat_loss = 0.3,
								children = empty_array(),
								equipped = empty_array(),
							},
						},
						equipped = empty_array(),
					},
				},
				equipped = empty_array(),
			},
		},
	}
end

-- AC012: damage_entity with Body distributes to parts, updates status
local function test_damage_entity_distributes_to_parts()
	local e = spawn_entity()
	set_component(e, "Body", make_humanoid_body())
	set_component(e, "Health", { current = 85.0, max = 85.0 })

	damage_entity(e, 85.0)
	run_native_system("BodyPartDamageSystem")

	local body = get_component(e, "Body")
	assert.equals(body.parts[1].status, "broken")
	assert.equals(body.parts[1].hp, 0.0)
end

-- AC013: damage_entity_part targets specific part
local function test_damage_entity_part_targets_specific()
	local e = spawn_entity()
	set_component(e, "Body", make_humanoid_body())
	set_component(e, "Health", { current = 85.0, max = 85.0 })

	damage_entity_part(e, "left hand", 5.0)
	run_native_system("BodyPartDamageSystem")

	local body = get_component(e, "Body")
	local hand = body.parts[1].children[1].children[1]
	assert.equals(hand.hp, 5.0)
	assert.equals(hand.status, "wounded")
end

-- AC014: get_body_part returns updated hp and max_hp fields
local function test_get_body_part_returns_hp_max_hp()
	local e = spawn_entity()
	set_component(e, "Body", make_humanoid_body())
	set_component(e, "Health", { current = 85.0, max = 85.0 })

	damage_entity_part(e, "left hand", 3.0)
	run_native_system("BodyPartDamageSystem")

	local hand = get_body_part(e, "left hand")
	assert.is_table(hand)
	assert.equals(hand.hp, 7.0)
	assert.equals(hand.max_hp, 10.0)
end

return {
	test_damage_entity_distributes_to_parts = test_damage_entity_distributes_to_parts,
	test_damage_entity_part_targets_specific = test_damage_entity_part_targets_specific,
	test_get_body_part_returns_hp_max_hp = test_get_body_part_returns_hp_max_hp,
}
