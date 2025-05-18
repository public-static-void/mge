TestPosition = {}

function TestPosition:test_position_component_and_move()
	local ok, err = pcall(function()
		local id = spawn_entity()
		set_component(id, "PositionComponent", { pos = { Square = { x = 0, y = 2, z = 0 } } })
		local pos = get_component(id, "PositionComponent")
		require("luaunit").assertEquals(pos.pos.Square.x, 0)
		require("luaunit").assertEquals(pos.pos.Square.y, 2)
		print("Before move: x=" .. tostring(pos.pos.Square.x))
		move_all(1, 0)
		local pos_after = get_component(id, "PositionComponent")
		print("After move: x=" .. tostring(pos_after.pos.Square.x))
		require("luaunit").assertEquals(pos_after.pos.Square.x, 1)
		require("luaunit").assertEquals(pos_after.pos.Square.y, 2)
	end)
	if not ok then
		print("TestPosition error:", err)
		error(err) -- Still fail the test, but now you'll see the real error!
	end
end

os.exit(require("luaunit").LuaUnit.run())
