add_cell(0, 0, 0)
add_cell(1, 0, 0)
add_cell(0, 1, 0)

local topo = get_map_topology_type()
assert(topo == "square", "Topology should be square")

local cells = get_all_cells()
assert(#cells >= 3, "Should have at least 3 cells")

local cell = { Square = { x = 0, y = 0, z = 0 } }
local neighbors = get_neighbors(cell)
assert(#neighbors > 0, "Cell should have neighbors")
