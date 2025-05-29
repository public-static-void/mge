local assert = require("assert")

local function test_entities_in_cell()
	local eid = spawn_entity()
	set_component(eid, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
	local cell = { Square = { x = 0, y = 0, z = 0 } }
	local entities = entities_in_cell(cell)
	assert.equals(#entities, 1, "Should find one entity in cell")
end

return {
	test_entities_in_cell = test_entities_in_cell,
}
