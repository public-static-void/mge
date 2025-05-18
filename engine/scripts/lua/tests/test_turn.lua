TestTurn = {}

function TestTurn:test_tick_advances_turn_and_state()
	local ok, err = pcall(function()
		local id = spawn_entity()
		set_component(id, "PositionComponent", { pos = { Square = { x = 0.0, y = 0.0, z = 0.0 } } })
		set_component(id, "Health", { current = 10.0, max = 10.0 })

		local pos_before = get_component(id, "PositionComponent")
		local health_before = get_component(id, "Health")
		local turn_before = get_turn()

		tick()

		local pos_after = get_component(id, "PositionComponent")
		local health_after = get_component(id, "Health")
		local turn_after = get_turn()

		print("Turn before:", turn_before, "Turn after:", turn_after)
		print("Health before:", health_before.current, "Health after:", health_after.current)
		require("luaunit").assertEquals(turn_after, turn_before + 1, "Turn should increment by 1 after tick()")
		require("luaunit").assertEquals(health_after.current, 9, "Health should decrease by 1 after tick()")
	end)
	if not ok then
		print("TestTurn error:", err)
		error(err)
	end
end

os.exit(require("luaunit").LuaUnit.run())
