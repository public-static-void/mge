local luaunit = require("luaunit")

TestWorldgen = {}

function TestWorldgen:test_register_and_invoke_worldgen()
	-- Register a Lua worldgen function
	register_worldgen("luagen", function(params)
		luaunit.assertEquals(type(params), "table")
		luaunit.assertEquals(params.width, 7)
		return { cells = { { id = "luacell", x = 1, y = 2 } } }
	end)

	-- List should include our plugin
	local names = list_worldgen()
	luaunit.assertEquals(type(names), "table")
	local found = false
	for _, name in ipairs(names) do
		if name == "luagen" then
			found = true
			break
		end
	end
	luaunit.assertTrue(found, "luagen should be in worldgen plugin list")

	-- Invocation should call our function and return the expected structure
	local result = invoke_worldgen("luagen", { width = 7 })
	luaunit.assertEquals(result.cells[1].id, "luacell")
end

os.exit(luaunit.LuaUnit.run())
