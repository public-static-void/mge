-- test_multiscale_map.lua: Parity test for the multi-scale map navigation API.
-- Exercises all 9 functions: register_map, set_active_map, get_map_names,
-- get_active_map_name, link_maps, enter_map, exit_map, map_cell, unmap_cell.
-- Each test gets a fresh World via the test runner.

local assert = require("assert")

local function contains(arr, value)
	for _, v in ipairs(arr) do
		if v == value then
			return true
		end
	end
	return false
end

-- Build map JSON tables without a literal `return {` so the test runner's
-- source-parsing discovery (first `return {` in the file) still finds the
-- final test table.
local function square_map_json()
	local map = {}
	map.topology = "square"
	map.cells = {}
	map.cells[1] = { x = 0, y = 0, z = 0 }
	map.cells[2] = { x = 1, y = 0, z = 0 }
	return map
end

local function province_map_json()
	local map = {}
	map.topology = "province"
	map.cells = {}
	map.cells[1] = { id = "prov_a" }
	map.cells[2] = { id = "prov_b" }
	return map
end

-- AC001: two named maps registered, both in get_map_names().
local function test_register_two_maps_appear_in_registry()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())

	local names = get_map_names()
	assert.is_true(contains(names, "overmap"), "overmap should be registered")
	assert.is_true(contains(names, "field"), "field should be registered")
	assert.equals(#names, 2, "Exactly two maps registered")
end

-- AC002: set_active_map reflects the selected map's topology.
local function test_set_active_map_reflects_topology()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())

	set_active_map("field")
	assert.equals(get_map_topology_type(), "square", "field map should be square")

	set_active_map("overmap")
	assert.equals(get_map_topology_type(), "province", "overmap should be province")
end

-- AC003: set_active_map on an unknown name errors and leaves active map unchanged.
local function test_set_active_map_unknown_name_errors_and_unchanged()
	register_map("field", square_map_json())
	set_active_map("field")

	local ok, err = pcall(set_active_map, "nonexistent")
	assert.is_false(ok, "set_active_map should error on unknown name")
	assert.is_true(
		string.find(tostring(err) or "", "not registered") ~= nil,
		"Error should mention not registered"
	)

	assert.equals(get_active_map_name(), "field", "Active map unchanged")
	assert.equals(get_map_topology_type(), "square", "Topology unchanged")
end

-- AC004: get_active_map_name returns the last selected map; get_map_names
-- contains exactly the registered names.
local function test_active_map_name_and_exact_names()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())

	assert.equals(get_active_map_name(), "", "No map selected initially")

	set_active_map("field")
	assert.equals(get_active_map_name(), "field", "Active map is field")

	set_active_map("overmap")
	assert.equals(get_active_map_name(), "overmap", "Active map is overmap")

	local names = get_map_names()
	assert.equals(#names, 2, "Exactly two registered names")
	assert.is_true(contains(names, "field"), "field in names")
	assert.is_true(contains(names, "overmap"), "overmap in names")
end

-- AC005: a square-topology map can serve as an overmap (topology/type decoupled).
local function test_square_topology_named_overmap()
	register_map("overmap", square_map_json())
	set_active_map("overmap")
	assert.equals(get_map_topology_type(), "square", "A square map named overmap is valid")
end

-- AC006: a province-topology map can serve as a strategic map.
local function test_province_topology_named_strategic()
	register_map("strategic", province_map_json())
	set_active_map("strategic")
	assert.equals(get_map_topology_type(), "province", "A province map named strategic is valid")
end

-- AC007: enter_map switches the active map and positions the camera at the entry cell.
local function test_enter_map_switches_active_and_positions_camera()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())
	set_active_map("overmap")

	link_maps("overmap", { Province = { id = "prov_a" } }, "field", { Square = { x = 1, y = 0, z = 0 } })

	enter_map("field", { Square = { x = 1, y = 0, z = 0 } })

	assert.equals(get_active_map_name(), "field", "Active map is field")
	assert.equals(get_map_topology_type(), "square", "Field map is square")

	local cam = get_camera()
	assert.equals(cam.x, 1, "Camera x at entry cell")
	assert.equals(cam.y, 0, "Camera y at entry cell")
	assert.equals(cam.z, 0, "Camera z at entry cell")
end

-- AC008: exit_map returns to the previously active map.
local function test_exit_map_returns_to_previous_map()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())
	set_active_map("overmap")

	enter_map("field", { Square = { x = 0, y = 0, z = 0 } })
	assert.equals(get_active_map_name(), "field", "Active map is field after enter")

	exit_map()
	assert.equals(get_active_map_name(), "overmap", "Active map is overmap after exit")
	assert.equals(get_map_topology_type(), "province", "Overmap topology is province")
end

-- AC009: enter_map on an unknown map errors; exit_map with an empty stack errors.
local function test_enter_unknown_map_and_exit_empty_stack_error()
	register_map("field", square_map_json())

	local ok, err = pcall(enter_map, "nonexistent", { Square = { x = 0, y = 0, z = 0 } })
	assert.is_false(ok, "enter_map should error on unknown map")
	assert.is_true(
		string.find(tostring(err) or "", "not registered") ~= nil,
		"Error should mention not registered"
	)

	local ok2, err2 = pcall(exit_map)
	assert.is_false(ok2, "exit_map should error with empty stack")
	assert.is_true(
		string.find(tostring(err2) or "", "no previous map") ~= nil,
		"Error should mention no previous map"
	)
end

-- AC010: map_cell/unmap_cell round-trip a linked source cell to its target cell and back.
local function test_map_cell_unmap_cell_round_trip()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())

	local source_cell = { Province = { id = "prov_a" } }
	local target_cell = { Square = { x = 2, y = 1, z = 0 } }
	link_maps("overmap", source_cell, "field", target_cell)

	local mapped = map_cell("overmap", source_cell)
	assert.table_equals(mapped, target_cell, "map_cell returns the linked target cell")

	local unmapped = unmap_cell("field", target_cell)
	assert.table_equals(unmapped, source_cell, "unmap_cell returns the linked source cell")
end

-- AC011: map_cell/unmap_cell return nil for unlinked source/target cells.
local function test_map_cell_unmap_cell_unlinked_returns_nil()
	register_map("overmap", province_map_json())
	register_map("field", square_map_json())

	link_maps("overmap", { Province = { id = "prov_a" } }, "field", { Square = { x = 2, y = 1, z = 0 } })

	assert.is_nil(map_cell("overmap", { Province = { id = "prov_b" } }), "Unlinked source cell returns nil")
	assert.is_nil(map_cell("field", { Square = { x = 0, y = 0, z = 0 } }), "Unlinked map returns nil")
	assert.is_nil(unmap_cell("field", { Square = { x = 9, y = 9, z = 0 } }), "Unlinked target cell returns nil")
	assert.is_nil(unmap_cell("overmap", { Province = { id = "prov_a" } }), "Unlinked target map returns nil")
end

-- AC012: coordinate mapping is topology-generic (province -> square round-trip).
local function test_cross_topology_round_trip_province_to_square()
	register_map("strategic", province_map_json())
	register_map("tactical", square_map_json())

	local source_cell = { Province = { id = "prov_b" } }
	local target_cell = { Square = { x = 3, y = 4, z = 0 } }
	link_maps("strategic", source_cell, "tactical", target_cell)

	assert.table_equals(map_cell("strategic", source_cell), target_cell, "Province cell maps to square cell")
	assert.table_equals(unmap_cell("tactical", target_cell), source_cell, "Square cell unmaps to province cell")
end

-- AC019: duplicate register_map errors (no panic).
local function test_register_duplicate_name_errors()
	register_map("field", square_map_json())

	local ok, err = pcall(register_map, "field", square_map_json())
	assert.is_false(ok, "register_map should error on duplicate name")
	assert.is_true(
		string.find(tostring(err) or "", "already registered") ~= nil,
		"Error should mention already registered"
	)
end

return {
	test_register_two_maps_appear_in_registry = test_register_two_maps_appear_in_registry,
	test_set_active_map_reflects_topology = test_set_active_map_reflects_topology,
	test_set_active_map_unknown_name_errors_and_unchanged = test_set_active_map_unknown_name_errors_and_unchanged,
	test_active_map_name_and_exact_names = test_active_map_name_and_exact_names,
	test_square_topology_named_overmap = test_square_topology_named_overmap,
	test_province_topology_named_strategic = test_province_topology_named_strategic,
	test_enter_map_switches_active_and_positions_camera = test_enter_map_switches_active_and_positions_camera,
	test_exit_map_returns_to_previous_map = test_exit_map_returns_to_previous_map,
	test_enter_unknown_map_and_exit_empty_stack_error = test_enter_unknown_map_and_exit_empty_stack_error,
	test_map_cell_unmap_cell_round_trip = test_map_cell_unmap_cell_round_trip,
	test_map_cell_unmap_cell_unlinked_returns_nil = test_map_cell_unmap_cell_unlinked_returns_nil,
	test_cross_topology_round_trip_province_to_square = test_cross_topology_round_trip_province_to_square,
	test_register_duplicate_name_errors = test_register_duplicate_name_errors,
}