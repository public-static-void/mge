local assert = require("assert")

local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

local function fix_arrays(obj)
	if type(obj) ~= "table" then
		return
	end
	if obj.equipped ~= nil then
		if type(obj.equipped) == "table" then
			if next(obj.equipped) == nil or (getmetatable(obj.equipped) and getmetatable(obj.equipped).__is_array) then
				obj.equipped = empty_array()
			else
				fix_arrays(obj.equipped)
			end
		end
	end
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
	set_body(e, { parts = empty_array() })

	-- Add torso
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
	assert.equals(#body.parts, 1)
	assert.equals(body.parts[1].name, "torso")

	-- Add left_arm as child of torso
	local torso = body.parts[1]
	if not torso.children then
		torso.children = empty_array()
	end
	table.insert(torso.children, {
		name = "left_arm",
		status = "healthy",
		kind = "flesh",
		temperature = 37.0,
		ideal_temperature = 37.0,
		insulation = 1.0,
		heat_loss = 0.1,
		children = empty_array(),
		equipped = empty_array(),
	})
	fix_arrays(body)
	set_body(e, body)
	body = get_body(e)
	assert.equals(#body.parts[1].children, 1)
	assert.equals(body.parts[1].children[1].name, "left_arm")

	-- Remove left_arm
	remove_body_part(e, "left_arm")
	body = get_body(e)
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
