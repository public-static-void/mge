local luaunit = require("luaunit")

TestTurn = {}

function TestTurn:test_tick_advances_turn_and_state()
	local id = spawn_entity()
	set_component(id, "PositionComponent", { pos = { Square = { x = 0.0, y = 0.0, z = 0.0 } } })
	set_component(id, "Health", { current = 10.0, max = 10.0 })

	-- Capture initial state
	local pos_before = get_component(id, "PositionComponent")
	local health_before = get_component(id, "Health")
	local turn_before = get_turn()

	-- Advance one tick
	tick()

	-- Capture state after tick
	local pos_after = get_component(id, "PositionComponent")
	local health_after = get_component(id, "Health")
	local turn_after = get_turn()

	-- Assertions (customize as needed based on what tick() should do)
	luaunit.assertEquals(pos_before.pos.Square.x, pos_after.pos.Square.x)
	luaunit.assertEquals(pos_before.pos.Square.y, pos_after.pos.Square.y)
	luaunit.assertEquals(health_before.current, health_after.current)
	luaunit.assertEquals(health_before.max, health_after.max)
	luaunit.assertEquals(turn_after, turn_before + 1, "Turn should increment by 1 after tick()")
end

os.exit(luaunit.LuaUnit.run())
