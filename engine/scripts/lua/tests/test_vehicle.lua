local assert = require("assert")
local utils = require("utils")

local function contains(arr, value)
	for _, v in ipairs(arr) do
		if v == value then
			return true
		end
	end
	return false
end

local function make_open_plane()
	local size = 10
	for x = -size, size do
		for y = -size, size do
			add_cell(x, y, 0)
		end
	end
	for x = -size, size do
		for y = -size, size do
			for dx = -1, 1 do
				for dy = -1, 1 do
					if dx ~= 0 or dy ~= 0 then
						local nx = x + dx
						local ny = y + dy
						if nx >= -size and nx <= size and ny >= -size and ny <= size then
							add_neighbor({ x = x, y = y, z = 0 }, { x = nx, y = ny, z = 0 })
						end
					end
				end
			end
		end
	end
end

local function make_corridor(len)
	for x = 0, len do
		add_cell(x, 0, 0)
	end
	for x = 0, len - 1 do
		add_neighbor({ x = x, y = 0, z = 0 }, { x = x + 1, y = 0, z = 0 })
		add_neighbor({ x = x + 1, y = 0, z = 0 }, { x = x, y = 0, z = 0 })
	end
end

local function square_cell(x, y)
	local coords = {}
	coords.x = x
	coords.y = y
	coords.z = 0
	local cell = {}
	cell.Square = coords
	return cell
end

local function spawn_vehicle(x, y, capacity, speed, blocked)
	local eid = spawn_entity()
	set_component(eid, "Position", { pos = square_cell(x, y) })
	local terrain_list = blocked or utils.empty_array()
	set_component(eid, "Vehicle", { capacity = capacity, speed = speed, blocked_terrains = terrain_list })
	return eid
end

local function spawn_rider(x, y)
	local eid = spawn_entity()
	set_component(eid, "Position", { pos = square_cell(x, y) })
	set_component(eid, "Agent", { entity_id = eid })
	return eid
end

local function pos_of(eid)
	return get_component(eid, "Position").pos
end

local function test_vehicle_schema_registered_with_defaults()
	local schema = get_component_schema("Vehicle")
	assert.not_nil(schema, "Vehicle schema must be retrievable")
	assert.equals(schema.title, "Vehicle", "schema title must be Vehicle")

	local eid = spawn_entity()
	set_component(eid, "Vehicle", { capacity = 2, speed = 1 })
	local stored = get_component(eid, "Vehicle")
	assert.equals(stored.capacity, 2, "capacity should round-trip")
	assert.equals(stored.speed, 1, "speed should round-trip")
end

local function test_embark_records_occupancy_clears_path_and_emits()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider = spawn_rider(0, 0)
	set_component(rider, "Agent", { entity_id = rider, move_path = { square_cell(1, 0) } })

	local ok, err = embark_vehicle(vehicle, rider)
	assert.is_true(ok, "embark should succeed")
	assert.is_nil(err, "err should be nil on success")

	local occupants = get_vehicle_occupants(vehicle)
	assert.equals(#occupants, 1, "one occupant expected")
	assert.equals(occupants[1], rider, "rider should be listed")
	assert.is_true(is_mounted(rider), "rider should be mounted")
	local agent = get_component(rider, "Agent")
	assert.is_true(agent.move_path == nil or #agent.move_path == 0, "embark must clear the rider path")

	update_event_buses()
	local events = poll_ecs_event("vehicle_embarked")
	assert.equals(#events, 1, "one vehicle_embarked event expected")
	assert.equals(events[1].vehicle, vehicle, "event vehicle mismatch")
	assert.equals(events[1].rider, rider, "event rider mismatch")
end

local function test_embark_rejects_bad_ids_and_double_mount()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local other = spawn_vehicle(1, 0, 2, 1)
	local rider = spawn_rider(0, 0)

	local ok1, err1 = embark_vehicle(9999, rider)
	assert.is_false(ok1, "unknown vehicle should fail")
	assert.equals(err1, "no_vehicle", "err should be no_vehicle")

	local ok2, err2 = embark_vehicle(vehicle, 9999)
	assert.is_false(ok2, "unknown rider should fail")
	assert.equals(err2, "no_rider", "err should be no_rider")

	local ok3, err3 = embark_vehicle(vehicle, other)
	assert.is_false(ok3, "vehicle-as-rider should fail")
	assert.equals(err3, "no_rider", "err should be no_rider")

	local ok4, _ = embark_vehicle(vehicle, rider)
	assert.is_true(ok4, "first embark should succeed")
	local ok5, err5 = embark_vehicle(other, rider)
	assert.is_false(ok5, "double mount should fail")
	assert.equals(err5, "already_mounted", "err should be already_mounted")
	assert.equals(#get_vehicle_occupants(other), 0, "no transfer on double mount")
end

local function test_disembark_colocates_rider_and_emits()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider)
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = utils.empty_array(),
		occupants = { rider },
		move_path = { square_cell(2, 0) },
	})
	tick()

	local ok, err = disembark_vehicle(rider)
	assert.is_true(ok, "disembark should succeed")
	assert.is_nil(err, "err should be nil on success")
	assert.is_false(is_mounted(rider), "rider should no longer be mounted")
	assert.equals(#get_vehicle_occupants(vehicle), 0, "occupants should be empty")
	local vpos = pos_of(vehicle)
	local rpos = pos_of(rider)
	assert.equals(rpos.Square.x, vpos.Square.x, "rider x must equal vehicle x")
	assert.equals(rpos.Square.y, vpos.Square.y, "rider y must equal vehicle y")

	update_event_buses()
	local events = poll_ecs_event("vehicle_disembarked")
	assert.equals(#events, 1, "one vehicle_disembarked event expected")

	local ok2, err2 = disembark_vehicle(rider)
	assert.is_false(ok2, "second disembark should fail")
	assert.equals(err2, "not_mounted", "err should be not_mounted")
end

local function test_comove_speed_one_with_two_riders()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider_a = spawn_rider(0, 0)
	local rider_b = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider_a)
	embark_vehicle(vehicle, rider_b)
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = utils.empty_array(),
		occupants = { rider_a, rider_b },
		move_path = { square_cell(1, 0), square_cell(2, 0), square_cell(3, 0) },
	})

	local expected = { 1, 2, 3 }
	for i, x in ipairs(expected) do
		tick()
		local vpos = pos_of(vehicle)
		assert.equals(vpos.Square.x, x, "vehicle x after tick " .. tostring(i))
		for _, rider in ipairs({ rider_a, rider_b }) do
			local rpos = pos_of(rider)
			assert.equals(rpos.Square.x, x, "rider x must track vehicle after tick " .. tostring(i))
			assert.equals(rpos.Square.y, 0, "rider y must track vehicle after tick " .. tostring(i))
		end
		local in_cell = entities_in_cell(square_cell(x, 0))
		for _, eid in ipairs({ vehicle, rider_a, rider_b }) do
			local found = false
			for _, id in ipairs(in_cell) do
				if id == eid then
					found = true
				end
			end
			assert.is_true(found, "entity " .. tostring(eid) .. " must be in the vehicle cell")
		end
	end
end

local function test_comove_speed_two()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 2)
	local rider = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider)
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 2,
		blocked_terrains = utils.empty_array(),
		occupants = { rider },
		move_path = { square_cell(1, 0), square_cell(2, 0) },
	})

	tick()

	local vpos = pos_of(vehicle)
	assert.equals(vpos.Square.x, 2, "speed 2 must advance two cells per tick")
	local rpos = pos_of(rider)
	assert.equals(rpos.Square.x, 2, "rider must track the speed-2 vehicle")
end

local function test_hex_comove_preserves_variant()
	apply_generated_map({
		topology = "hex",
		cells = {
			{ q = 0, r = 0, z = 0 },
			{ q = 1, r = 0, z = 0 },
		},
	})
	add_neighbor({ 0, 0, 0 }, { 1, 0, 0 })
	add_neighbor({ 1, 0, 0 }, { 0, 0, 0 })

	local vehicle = spawn_entity()
	set_component(vehicle, "Position", { pos = { Hex = { q = 0, r = 0, z = 0 } } })
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = utils.empty_array(),
		occupants = utils.empty_array(),
		move_path = { { Hex = { q = 1, r = 0, z = 0 } } },
	})
	local rider = spawn_entity()
	set_component(rider, "Position", { pos = { Hex = { q = 0, r = 0, z = 0 } } })
	embark_vehicle(vehicle, rider)

	tick()

	local vpos = pos_of(vehicle)
	assert.equals(vpos.Hex.q, 1, "hex vehicle must advance one step")
	local rpos = pos_of(rider)
	assert.equals(rpos.Hex.q, 1, "hex rider must track the vehicle")
	assert.not_nil(rpos.Hex, "rider Position must stay Hex-variant")
end

local function test_capacity_reject_emits_rejection()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 1, 1)
	local rider_a = spawn_rider(0, 0)
	local rider_b = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider_a)

	local ok, err = embark_vehicle(vehicle, rider_b)
	assert.is_false(ok, "second embark on capacity-1 vehicle should fail")
	assert.equals(err, "full", "err should be full")
	assert.equals(#get_vehicle_occupants(vehicle), 1, "occupants must be unchanged")
	assert.is_false(is_mounted(rider_b), "rejected rider must not be mounted")

	update_event_buses()
	local events = poll_ecs_event("vehicle_embark_rejected")
	assert.equals(#events, 1, "one rejection event expected")
	assert.equals(events[1].reason, "full", "rejection reason must be full")
end

local function test_terrain_guards_truncate_and_emit_blocked()
	make_corridor(4)
	local vehicle = spawn_vehicle(0, 0, 2, 1, { "water" })
	local rider = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider)
	set_cell_metadata(square_cell(1, 0), { walkable = false })
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = { "water" },
		occupants = { rider },
		move_path = { square_cell(1, 0), square_cell(2, 0) },
	})

	tick()

	local vpos = pos_of(vehicle)
	assert.equals(vpos.Square.x, 0, "vehicle must hold before the unwalkable step")
	local rpos = pos_of(rider)
	assert.equals(rpos.Square.x, 0, "rider must hold with the vehicle")
	local blocked = poll_ecs_event("vehicle_move_blocked")
	assert.equals(#blocked, 1, "one vehicle_move_blocked event expected")
end

local function test_assign_path_prefix_and_errors()
	make_corridor(4)
	local vehicle = spawn_vehicle(0, 0, 2, 1, { "water" })
	set_cell_metadata(square_cell(2, 0), { terrain = "water" })

	local steps = assign_vehicle_path(vehicle, square_cell(3, 0))
	assert.equals(steps, 1, "stored path must be the pre-blockage prefix")
	local stored = get_component(vehicle, "Vehicle")
	assert.equals(#stored.move_path, 1, "one prefix step must be stored")

	set_cell_metadata(square_cell(1, 0), { terrain = "water" })
	local steps_empty = assign_vehicle_path(vehicle, square_cell(3, 0))
	assert.equals(steps_empty, 0, "fully-blocked goal must store an empty path")

	local ok, err = pcall(assign_vehicle_path, 9999, square_cell(1, 0))
	assert.is_false(ok, "assign on unknown vehicle should error")
	assert.is_true(string.find(tostring(err), "no_vehicle") ~= nil, "error should mention no_vehicle")
end

local function test_mounted_rider_path_suppressed()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider)
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = utils.empty_array(),
		occupants = { rider },
		move_path = { square_cell(1, 0), square_cell(2, 0), square_cell(3, 0) },
	})

	for i = 1, 3 do
		set_component(rider, "Agent", { entity_id = rider, move_path = { square_cell(0, 5) } })
		tick()
		local vpos = pos_of(vehicle)
		local rpos = pos_of(rider)
		assert.equals(rpos.Square.x, vpos.Square.x, "rider must track vehicle on tick " .. tostring(i))
		assert.equals(rpos.Square.y, vpos.Square.y, "rider must track vehicle on tick " .. tostring(i))
	end
end

local function test_despawn_leaves_rider_alive_at_last_cell()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider)
	local vpos = pos_of(vehicle)

	despawn_entity(vehicle)
	tick()

	assert.is_true(contains(get_entities(), rider), "rider must survive the vehicle")
	assert.is_false(is_mounted(rider), "rider must not be mounted after despawn")
	local rpos = pos_of(rider)
	assert.equals(rpos.Square.x, vpos.Square.x, "rider must keep the last vehicle cell")
end

local function province_map()
	local map = {}
	map.topology = "province"
	map.cells = {}
	map.cells[1] = { id = "prov_a" }
	map.cells[2] = { id = "prov_b" }
	return map
end

local function test_province_vehicle_holds_with_riders_synced()
	register_map("overmap", province_map())
	set_active_map("overmap")

	local vehicle = spawn_entity()
	set_component(vehicle, "Position", { pos = { Province = { id = "prov_a" } } })
	set_component(vehicle, "Vehicle", {
		capacity = 2,
		speed = 1,
		blocked_terrains = utils.empty_array(),
		occupants = utils.empty_array(),
		move_path = { { Province = { id = "prov_b" } } },
	})
	local rider = spawn_entity()
	set_component(rider, "Position", { pos = { Province = { id = "prov_a" } } })
	embark_vehicle(vehicle, rider)

	tick()

	local vpos = pos_of(vehicle)
	assert.equals(vpos.Province.id, "prov_a", "province vehicle must hold position")
	local rpos = pos_of(rider)
	assert.equals(rpos.Province.id, "prov_a", "province rider must stay synced")
end

local function test_save_load_roundtrip_preserves_mounted_vehicle()
	make_open_plane()
	local vehicle = spawn_vehicle(0, 0, 2, 1)
	local rider_a = spawn_rider(0, 0)
	local rider_b = spawn_rider(0, 0)
	embark_vehicle(vehicle, rider_a)
	embark_vehicle(vehicle, rider_b)
	assign_vehicle_path(vehicle, square_cell(4, 0))
	tick()
	tick()

	save_to_file("test_vehicle_save.json")
	local entities = get_entities()
	for _, eid in ipairs(entities) do
		despawn_entity(eid)
	end
	load_from_file("test_vehicle_save.json")

	local occupants = get_vehicle_occupants(vehicle)
	assert.equals(#occupants, 2, "two riders must survive round-trip")
	assert.is_true(is_mounted(rider_a), "rider_a must stay mounted")
	assert.is_true(is_mounted(rider_b), "rider_b must stay mounted")
	local vpos = pos_of(vehicle)
	for _, rider in ipairs({ rider_a, rider_b }) do
		local rpos = pos_of(rider)
		assert.equals(rpos.Square.x, vpos.Square.x, "rider must stay co-located after load")
		assert.equals(rpos.Square.y, vpos.Square.y, "rider must stay co-located after load")
	end
end

return {
	test_vehicle_schema_registered_with_defaults = test_vehicle_schema_registered_with_defaults,
	test_embark_records_occupancy_clears_path_and_emits = test_embark_records_occupancy_clears_path_and_emits,
	test_embark_rejects_bad_ids_and_double_mount = test_embark_rejects_bad_ids_and_double_mount,
	test_disembark_colocates_rider_and_emits = test_disembark_colocates_rider_and_emits,
	test_comove_speed_one_with_two_riders = test_comove_speed_one_with_two_riders,
	test_comove_speed_two = test_comove_speed_two,
	test_hex_comove_preserves_variant = test_hex_comove_preserves_variant,
	test_capacity_reject_emits_rejection = test_capacity_reject_emits_rejection,
	test_terrain_guards_truncate_and_emit_blocked = test_terrain_guards_truncate_and_emit_blocked,
	test_assign_path_prefix_and_errors = test_assign_path_prefix_and_errors,
	test_mounted_rider_path_suppressed = test_mounted_rider_path_suppressed,
	test_despawn_leaves_rider_alive_at_last_cell = test_despawn_leaves_rider_alive_at_last_cell,
	test_province_vehicle_holds_with_riders_synced = test_province_vehicle_holds_with_riders_synced,
	test_save_load_roundtrip_preserves_mounted_vehicle = test_save_load_roundtrip_preserves_mounted_vehicle,
}
