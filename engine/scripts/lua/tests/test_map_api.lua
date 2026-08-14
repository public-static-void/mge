local assert = require("assert")

local function contains(arr, value)
	for _, v in ipairs(arr) do
		if v == value then
			return true
		end
	end
	return false
end

local function test_map_api()
	add_cell(0, 0, 0)
	add_cell(1, 0, 0)
	add_cell(0, 1, 0)

	-- Add neighbors explicitly
	add_neighbor({ 0, 0, 0 }, { 1, 0, 0 })
	add_neighbor({ 0, 0, 0 }, { 0, 1, 0 })

	local topo = get_map_topology_type()
	assert.equals(topo, "square", "Topology should be square")

	local cells = get_all_cells()
	assert.is_true(#cells >= 3, "Should have at least 3 cells")

	local cell = { Square = { x = 0, y = 0, z = 0 } }
	local neighbors = get_neighbors(cell)
	assert.is_true(#neighbors > 0, "Cell should have neighbors")
end

local function test_entities_in_zlevel_returns_matching_entities()
	add_cell(2, 0, 1)

	local lower = spawn_entity()
	set_component(lower, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
	local upper = spawn_entity()
	set_component(upper, "Position", { pos = { Square = { x = 2, y = 0, z = 1 } } })

	local on_zero = entities_in_zlevel(0)
	assert.is_true(contains(on_zero, lower), "Lower entity should be on z-level 0")
	assert.is_false(contains(on_zero, upper), "Upper entity should not be on z-level 0")

	local on_one = entities_in_zlevel(1)
	assert.is_true(contains(on_one, upper), "Upper entity should be on z-level 1")
	assert.is_false(contains(on_one, lower), "Lower entity should not be on z-level 1")
end

local function test_move_entity_3d_changes_z()
	add_cell(0, 0, 0)
	add_cell(0, 0, 2)

	local eid = spawn_entity()
	set_component(eid, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })

	move_entity_3d(eid, 0, 0, 2)

	local pos = get_component(eid, "Position")
	assert.equals(pos.pos.Square.x, 0, "x should be unchanged by vertical movement")
	assert.equals(pos.pos.Square.y, 0, "y should be unchanged by vertical movement")
	assert.equals(pos.pos.Square.z, 2, "z should increase by dz")

	assert.is_true(contains(entities_in_zlevel(2), eid), "Entity should be found on new z-level")
	assert.is_false(contains(entities_in_zlevel(0), eid), "Entity should leave old z-level")
end

local function test_move_entity_3d_shifts_xy_and_z()
	local eid = spawn_entity()
	set_component(eid, "Position", { pos = { Square = { x = 1, y = 1, z = 0 } } })

	move_entity_3d(eid, 2, 3, 1)

	local pos = get_component(eid, "Position")
	assert.equals(pos.pos.Square.x, 3, "x should shift by dx")
	assert.equals(pos.pos.Square.y, 4, "y should shift by dy")
	assert.equals(pos.pos.Square.z, 1, "z should shift by dz")
end

return {
	test_map_api = test_map_api,
	test_entities_in_zlevel_returns_matching_entities = test_entities_in_zlevel_returns_matching_entities,
	test_move_entity_3d_changes_z = test_move_entity_3d_changes_z,
	test_move_entity_3d_shifts_xy_and_z = test_move_entity_3d_shifts_xy_and_z,
}
