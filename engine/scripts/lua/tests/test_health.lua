local luaunit = require("luaunit")

TestHealth = {}

function TestHealth:test_health_component()
	local id = spawn_entity()
	set_component(id, "Health", { current = 7.0, max = 10.0 })
	local health = get_component(id, "Health")
	luaunit.assertAlmostEquals(health.current, 7.0, 1e-5)
	luaunit.assertAlmostEquals(health.max, 10.0, 1e-5)
end

os.exit(luaunit.LuaUnit.run())
