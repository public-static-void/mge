local assert = require("assert")

array_mt = { __is_array = true }

local function test_register_and_invoke_worldgen()
	register_worldgen_plugin("luagen", function(params)
		assert.equals(type(params), "table")
		assert.equals(params.width, 7)
		local neighbors = {}
		setmetatable(neighbors, array_mt)
		return {
			topology = "square",
			cells = {
				{ x = 1, y = 2, z = 0, neighbors = neighbors },
			},
		}
	end)

	local names = list_worldgen_plugins()
	assert.equals(type(names), "table")
	local found = false
	for _, name in ipairs(names) do
		if name == "luagen" then
			found = true
			break
		end
	end
	assert.is_true(found, "luagen should be in worldgen plugin list")

	local result = invoke_worldgen_plugin("luagen", { width = 7 })
	assert.equals(result.topology, "square")
	assert.equals(result.cells[1].x, 1)
	assert.equals(result.cells[1].y, 2)
	assert.equals(result.cells[1].z, 0)
	assert.is_table(result.cells[1].neighbors)
end

local function test_lua_validator_and_postprocessor()
	local called_validator = false
	local called_post = false

	register_worldgen_validator(function(map)
		called_validator = true
		assert.equals(map.topology, "square")
		return true
	end)
	register_worldgen_postprocessor(function(map)
		called_post = true
		map.lua_post = 42
	end)

	register_worldgen_plugin("luagen2", function(params)
		local neighbors = {}
		setmetatable(neighbors, array_mt)
		return {
			topology = "square",
			cells = {
				{ x = 0, y = 0, z = 0, neighbors = neighbors },
			},
		}
	end)

	local result = invoke_worldgen_plugin("luagen2", {})
	assert.is_true(called_validator)
	assert.is_true(called_post)
	assert.equals(result.lua_post, 42)
end

return {
	test_register_and_invoke_worldgen = test_register_and_invoke_worldgen,
	test_lua_validator_and_postprocessor = test_lua_validator_and_postprocessor,
}
