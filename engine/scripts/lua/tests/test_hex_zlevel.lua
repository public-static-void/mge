local assert = require("assert")

local function contains(arr, value)
	for _, v in ipairs(arr) do
		if v == value then
			return true
		end
	end
	return false
end

-- Search a list of serde CellKey tables for a Hex cell with matching q/r/z.
-- get_all_cells()/get_neighbors() return tables like { Hex = { q = 1, r = 2, z = 3 } }.
local function contains_hex_cell(cells, q, r, z)
	for _, cell in ipairs(cells) do
		local hex = cell.Hex
		if hex and hex.q == q and hex.r == r and hex.z == z then
			return true
		end
	end
	return false
end

local function test_hex_add_cell()
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 0, r = 0, z = 0 },
		},
	})
	assert.equals(get_map_topology_type(), "hex", "Topology should be hex after apply_generated_map")

	add_cell(1, 2, 3)

	local cells = get_all_cells()
	assert.is_true(contains_hex_cell(cells, 0, 0, 0), "apply_generated_map should have added Hex(0, 0, 0)")
	assert.is_true(contains_hex_cell(cells, 1, 2, 3), "add_cell should add a Hex cell as (q, r, z)")
end

local function test_hex_add_neighbor()
	-- Cells are not 6-adjacent, so no neighbor is inferred: the explicit
	-- add_neighbor edge is the only way the pair can appear.
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 0, r = 0, z = 0 },
			{ q = 5, r = 5, z = 0 },
		},
	})

	add_neighbor({ 0, 0, 0 }, { 5, 5, 0 })

	local neighbors = get_neighbors({ Hex = { q = 0, r = 0, z = 0 } })
	assert.is_true(contains_hex_cell(neighbors, 5, 5, 0), "add_neighbor should add a hex neighbor edge")
end

local function test_hex_entities_in_zlevel()
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 0, r = 0, z = 0 },
			{ q = 2, r = 0, z = 1 },
		},
	})

	local lower = spawn_entity()
	set_component(lower, "Position", { pos = { Hex = { q = 0, r = 0, z = 0 } } })
	local upper = spawn_entity()
	set_component(upper, "Position", { pos = { Hex = { q = 2, r = 0, z = 1 } } })

	local on_zero = entities_in_zlevel(0)
	assert.is_true(contains(on_zero, lower), "Lower entity should be on z-level 0")
	assert.is_false(contains(on_zero, upper), "Upper entity should not be on z-level 0")

	local on_one = entities_in_zlevel(1)
	assert.is_true(contains(on_one, upper), "Upper entity should be on z-level 1")
	assert.is_false(contains(on_one, lower), "Lower entity should not be on z-level 1")
end

local function test_hex_move_entity_3d()
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 1, r = 1, z = 0 },
		},
	})

	local eid = spawn_entity()
	set_component(eid, "Position", { pos = { Hex = { q = 1, r = 1, z = 0 } } })

	move_entity_3d(eid, 2, 3, 1)

	local pos = get_component(eid, "Position")
	assert.equals(pos.pos.Hex.q, 3, "q should shift by dx")
	assert.equals(pos.pos.Hex.r, 4, "r should shift by dy")
	assert.equals(pos.pos.Hex.z, 1, "z should shift by dz")

	assert.is_true(contains(entities_in_zlevel(1), eid), "Entity should be found on new z-level")
end

local function test_hex_camera()
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 0, r = 0, z = 0 },
		},
	})

	set_camera(3, 7, 2)

	local cam = get_camera()
	assert.equals(cam.x, 3, "Camera x should be 3 after set_camera on a hex map")
	assert.equals(cam.y, 7, "Camera y should be 7 after set_camera on a hex map")
	assert.equals(cam.z, 2, "Camera z should be 2 after set_camera on a hex map")

	local camera_id = get_entities_with_component("Camera")[1]
	assert.is_true(camera_id ~= nil, "set_camera should create a camera entity")
	local pos = get_component(camera_id, "Position")
	assert.equals(pos.pos.Hex.q, 3, "Position should store pos.Hex.q on a hex map")
	assert.equals(pos.pos.Hex.r, 7, "Position should store pos.Hex.r on a hex map")
	assert.equals(pos.pos.Hex.z, 2, "Position should store pos.Hex.z on a hex map")
end

return {
	test_hex_add_cell = test_hex_add_cell,
	test_hex_add_neighbor = test_hex_add_neighbor,
	test_hex_entities_in_zlevel = test_hex_entities_in_zlevel,
	test_hex_move_entity_3d = test_hex_move_entity_3d,
	test_hex_camera = test_hex_camera,
}
