local assert = require("assert")

local function test_apply_generated_map()
	-- Generate a map using a built-in or test plugin
	local map = invoke_worldgen_plugin("simple_square", { width = 4, height = 4, z_levels = 1, seed = 42 })
	-- Apply it to the world
	world:apply_generated_map(map)
	-- Now check that the world has a map and it has expected properties
	local topology = world:get_map_topology_type()
	assert.equals(topology, "square")
	local cell_count = world:get_map_cell_count()
	assert.equals(cell_count, 16)
end

return {
	test_apply_generated_map = test_apply_generated_map,
}
