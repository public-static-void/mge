local assert = require("assert")

function test_map_metadata()
	add_cell(1, 2, 0)
	set_cell_metadata({ x = 1, y = 2, z = 0 }, { biome = "Forest", terrain = "Grass" })
	local meta = get_cell_metadata({ x = 1, y = 2, z = 0 })
	assert.equals(meta.biome, "Forest")
	assert.equals(meta.terrain, "Grass")
end

return { test_map_metadata = test_map_metadata }
