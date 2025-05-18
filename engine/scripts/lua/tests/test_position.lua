local luaunit = require("luaunit")

TestPosition = {}

function TestPosition:test_position_component_and_move()
	local id = spawn_entity()
	set_component(id, "PositionComponent", { pos = { Square = { x = 0, y = 2, z = 0 } } })
	local pos = get_component(id, "PositionComponent")
	luaunit.assertEquals(pos.pos.Square.x, 0)
	luaunit.assertEquals(pos.pos.Square.y, 2)

	-- Optionally, check print_positions() output if it returns a string
	-- print_positions()

	move_all(1, 0)
	local pos_after = get_component(id, "PositionComponent")
	luaunit.assertEquals(pos_after.pos.Square.x, 1)
	luaunit.assertEquals(pos_after.pos.Square.y, 2)

	-- Optionally, check print_positions() output if it returns a string
	-- print_positions()
end

os.exit(luaunit.LuaUnit.run())
