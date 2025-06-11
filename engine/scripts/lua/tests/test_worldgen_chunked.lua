local assert = require("assert")

local function test_generate_and_apply_chunk()
	local params = { width = 2, height = 2, z_levels = 1, seed = 123, chunk_x = 0, chunk_y = 0 }
	local chunk = invoke_worldgen_plugin("simple_square", params)
	assert.equals(#chunk.cells, 4)
	world:apply_generated_map(chunk)
	assert.equals(world:get_map_cell_count(), 4)

	local params2 = { width = 2, height = 2, z_levels = 1, seed = 456, chunk_x = 2, chunk_y = 0 }
	local chunk2 = invoke_worldgen_plugin("simple_square", params2)
	world:apply_chunk(chunk2)
	assert.equals(world:get_map_cell_count(), 8)
end

local function test_schema_validation_rejects_invalid_map()
	local invalid_map = { topology = "square", cells = { { x = 0, y = 0 } } }
	local ok, err = pcall(function()
		world:apply_generated_map(invalid_map)
	end)
	assert.is_false(ok)
end

return {
	test_generate_and_apply_chunk = test_generate_and_apply_chunk,
	test_schema_validation_rejects_invalid_map = test_schema_validation_rejects_invalid_map,
}
