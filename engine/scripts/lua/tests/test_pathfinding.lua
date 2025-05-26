local assert = require("assert")

local function test_pathfinding()
	-- Build a 3x3 grid
	for x = 0, 2 do
		for y = 0, 2 do
			add_cell(x, y, 0)
		end
	end
	for x = 0, 2 do
		for y = 0, 2 do
			for _, d in ipairs({ { 1, 0 }, { 0, 1 }, { -1, 0 }, { 0, -1 } }) do
				local nx, ny = x + d[1], y + d[2]
				if nx >= 0 and nx <= 2 and ny >= 0 and ny <= 2 then
					add_neighbor({ x = x, y = y, z = 0 }, { x = nx, y = ny, z = 0 })
				end
			end
		end
	end
	-- Block (1,1,0)
	set_cell_metadata({ x = 1, y = 1, z = 0 }, { walkable = false })
	local result = find_path({ x = 0, y = 0, z = 0 }, { x = 2, y = 2, z = 0 })
	assert.is_table(result)
	assert.is_table(result.path)
	for _, cell in ipairs(result.path) do
		assert.is_false(cell.x == 1 and cell.y == 1)
	end
	assert.equals(#result.path, 5)
end

return { test_pathfinding = test_pathfinding }
