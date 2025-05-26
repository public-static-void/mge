local assert = require("assert")

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

return {
	test_map_api = test_map_api,
}
