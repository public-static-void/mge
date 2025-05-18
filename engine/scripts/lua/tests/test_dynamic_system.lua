local luaunit = require("luaunit")

TestDynamicSystem = {}

function TestDynamicSystem:test_dynamic_system_runs()
	local ran = false
	register_system("test_lua_system", function()
		ran = true
	end)

	run_system("test_lua_system")
	luaunit.assertTrue(ran, "Lua dynamic system did not run!")
end

os.exit(luaunit.LuaUnit.run())
