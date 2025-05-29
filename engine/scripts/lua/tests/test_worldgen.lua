local assert = require("assert")

local function test_register_and_invoke_worldgen()
	-- Register a Lua worldgen function
	register_worldgen_plugin("luagen", function(params)
		assert.equals(type(params), "table")
		assert.equals(params.width, 7)
		return { cells = { { id = "luacell", x = 1, y = 2 } } }
	end)

	-- List should include our plugin
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

	-- Invocation should call our function and return the expected structure
	local result = invoke_worldgen_plugin("luagen", { width = 7 })
	assert.equals(result.cells[1].id, "luacell")
end

return {
	test_register_and_invoke_worldgen = test_register_and_invoke_worldgen,
}
