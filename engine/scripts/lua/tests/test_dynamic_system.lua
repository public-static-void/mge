local assert = require("assert")

local function test_dynamic_system_runs()
	local ran = false
	register_system("test_lua_system", function()
		ran = true
	end)

	run_system("test_lua_system")
	assert.is_true(ran, "Lua dynamic system did not run!")
end

return {
	test_dynamic_system_runs = test_dynamic_system_runs,
}
