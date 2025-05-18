local ran = false
register_system("test_lua_system", function()
	ran = true
end)

run_system("test_lua_system")
assert(ran == true, "Lua dynamic system did not run!")
