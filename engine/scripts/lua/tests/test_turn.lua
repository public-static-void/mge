local assert = require("assert")

local function test_tick_runs_registered_system()
	local id = spawn_entity()
	set_component(id, "Position", { pos = { Square = { x = 5, y = 7, z = 0 } } })

	-- Register a system that moves all positions by +1 x, +2 y
	register_system("MoveAll", function()
		for _, eid in ipairs(get_entities_with_component("Position")) do
			local pos = get_component(eid, "Position")
			pos.pos.Square.x = pos.pos.Square.x + 1
			pos.pos.Square.y = pos.pos.Square.y + 2
			set_component(eid, "Position", pos)
		end
	end)

	local pos_before = get_component(id, "Position")
	local turn_before = get_turn()

	tick()

	local pos_after = get_component(id, "Position")
	local turn_after = get_turn()

	assert.equals(turn_after, turn_before + 1, "Turn should increment by 1 after tick()")
	assert.equals(pos_after.pos.Square.x, pos_before.pos.Square.x + 1, "X should increment by 1")
	assert.equals(pos_after.pos.Square.y, pos_before.pos.Square.y + 2, "Y should increment by 2")
end

return {
	test_tick_runs_registered_system = test_tick_runs_registered_system,
}
