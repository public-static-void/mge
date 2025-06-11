local assert = require("assert")

local function test_register_and_invoke_worldgen()
	register_worldgen_plugin("luagen", function(params)
		assert.equals(type(params), "table")
		assert.equals(params.width, 7)
		local neighbors = {}
		setmetatable(neighbors, array_mt) -- Mark as array for Rust serialization
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

return {
	test_register_and_invoke_worldgen = test_register_and_invoke_worldgen,
}
