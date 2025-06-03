local assert = require("assert")

local function test_position_component_and_move()
	local id = spawn_entity()
	set_component(id, "Position", { pos = { Square = { x = 0, y = 2, z = 0 } } })
	local pos = get_component(id, "Position")
	assert.equals(pos.pos.Square.x, 0)
	assert.equals(pos.pos.Square.y, 2)
	for _, eid in ipairs(get_entities_with_component("Position")) do
		local pos = get_component(eid, "Position")
		pos.pos.Square.x = pos.pos.Square.x + 1
		set_component(eid, "Position", pos)
	end
	local pos_after = get_component(id, "Position")
	assert.equals(pos_after.pos.Square.x, 1)
	assert.equals(pos_after.pos.Square.y, 2)
end

return {
	test_position_component_and_move = test_position_component_and_move,
}
