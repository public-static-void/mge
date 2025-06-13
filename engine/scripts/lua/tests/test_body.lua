local assert = require("assert")

local function dump_table(tbl, indent)
	indent = indent or 0
	local formatting = string.rep("  ", indent)
	if type(tbl) ~= "table" then
		print(formatting .. tostring(tbl))
		return
	end
	print(formatting .. "{")
	for k, v in pairs(tbl) do
		io.write(formatting .. "  [" .. tostring(k) .. "] = ")
		if type(v) == "table" then
			dump_table(v, indent + 1)
		else
			print(tostring(v))
		end
	end
	print(formatting .. "}")
end

local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

local function test_body_get_set()
	local e = spawn_entity()
	local body = {
		parts = {
			{
				name = "torso",
				status = "healthy",
				kind = "flesh",
				temperature = 37.0,
				ideal_temperature = 37.0,
				insulation = 1.0,
				heat_loss = 0.1,
				children = empty_array(),
				equipped = empty_array(),
			},
		},
	}
	set_body(e, body)
	local got = get_body(e)
	assert.is_table(got, "get_body should return a table")
	assert.equals(got.parts[1].name, "torso")
end

local function test_body_add_remove_part()
	local e = spawn_entity()

	local function empty_array()
		local t = {}
		return setmetatable(t, { __is_array = true })
	end

	local left_arm = {
		name = "left_arm",
		status = "healthy",
		kind = "flesh",
		temperature = 37.0,
		ideal_temperature = 37.0,
		insulation = 1.0,
		heat_loss = 0.1,
		children = empty_array(),
		equipped = empty_array(),
	}
	local torso = {
		name = "torso",
		status = "healthy",
		kind = "flesh",
		temperature = 37.0,
		ideal_temperature = 37.0,
		insulation = 1.0,
		heat_loss = 0.1,
		children = empty_array(),
		equipped = empty_array(),
	}
	table.insert(torso.children, left_arm)

	local body = { parts = empty_array() }
	table.insert(body.parts, torso)

	set_body(e, body)
	body = get_body(e)
	assert.equals(#body.parts, 1)
	assert.equals(body.parts[1].name, "torso")
	assert.equals(#body.parts[1].children, 1)
	assert.equals(body.parts[1].children[1].name, "left_arm")

	remove_body_part(e, "left_arm")
	body = get_body(e)
	assert.equals(#body.parts, 1)
	assert.is_table(body.parts[1], "Torso should still be present after removing left_arm")
	assert.is_table(body.parts[1].children, "Torso children should be present after removing left_arm")
	assert.equals(#body.parts[1].children, 0)
end

local function test_body_get_body_part()
	local e = spawn_entity()
	set_body(e, { parts = empty_array() })
	add_body_part(e, {
		name = "torso",
		status = "healthy",
		kind = "flesh",
		temperature = 37.0,
		ideal_temperature = 37.0,
		insulation = 1.0,
		heat_loss = 0.1,
		children = empty_array(),
		equipped = empty_array(),
	})
	local body = get_body(e)
	local torso = get_body_part(e, "torso")
	assert.is_table(torso)
	assert.equals(torso.name, "torso")
	assert.equals(torso.status, "healthy")
end

return {
	test_body_get_set = test_body_get_set,
	test_body_add_remove_part = test_body_add_remove_part,
	test_body_get_body_part = test_body_get_body_part,
}
