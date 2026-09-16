local assert = require("assert")

local function setup_build_world()
	set_mode("colony")
	add_cell(0, 0, 0)
	add_cell(1, 0, 0)
	add_cell(2, 0, 0)

	local agent = spawn_entity()
	set_component(agent, "Agent", { entity_id = agent, state = "idle" })
	set_component(agent, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
	set_component(agent, "Inventory", {
		max_weight = 100.0,
		max_slots = 10,
		max_volume = 100.0,
		weight = 0.0,
		slots = {},
		volume = 0.0,
	})

	local stockpile = spawn_entity()
	set_component(stockpile, "Stockpile", { resources = { wood = 2 } })
	set_component(stockpile, "Position", { pos = { Square = { x = 1, y = 0, z = 0 } } })

	return agent, stockpile
end

local function find_construction_job(site)
	local jobs = find_jobs({ category = "construction" })
	for _, job in ipairs(jobs) do
		if job.target == site then
			return job.id
		end
	end
	return nil
end

local function deliver_to_site(agent, stockpile, site)
	local job_id = find_construction_job(site)
	assert.not_nil(job_id, "Construction job should be posted for the site")
	assert.equals(true, reserve_job_resources(job_id), "Reservation should succeed")
	local job = get_job(job_id)
	job.assigned_to = agent
	job.delivered_resources = { { kind = "wood", amount = 2 } }
	set_component(job_id, "Job", job)
	set_component(agent, "Position", { pos = { Square = { x = 2, y = 0, z = 0 } } })
	return job_id
end

local function test_place_blueprint_and_query_state()
	local agent, stockpile = setup_build_world()

	local site = place_blueprint("hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 2 } }, 2)
	assert.not_nil(site, "place_blueprint should return a site id")

	local state = get_construction_state(site)
	assert.equals(state.state, "pending")
	assert.equals(state.progress, 0)
	assert.equals(state.required_work, 2)
	assert.equals(state.building_type, "hut")

	local ghost = get_component(site, "ConstructionSite")
	assert.not_nil(ghost, "Ghost entity should carry ConstructionSite")
	local job_id = find_construction_job(site)
	assert.not_nil(job_id, "Linked construction job should exist")
	local job = get_job(job_id)
	assert.equals(job.category, "construction")
	assert.equals(job.job_type, "construct")
end

local function test_blueprint_validation_rejects_bad_input()
	setup_build_world()

	local ok_bounds = pcall(place_blueprint, "hut", { Square = { x = 9, y = 9, z = 0 } }, { { kind = "wood", amount = 1 } }, 1)
	assert.is_false(ok_bounds, "Out-of-bounds cell should be rejected")

	local site = place_blueprint("hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 1 } }, 1)
	assert.not_nil(site, "First blueprint should place")
	local ok_dup = pcall(place_blueprint, "hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 1 } }, 1)
	assert.is_false(ok_dup, "Duplicate blueprint on the occupied cell should be rejected")

	local ok_empty =
		pcall(place_blueprint, "hut", { Square = { x = 0, y = 0, z = 0 } }, {}, 1)
	assert.is_false(ok_empty, "Empty material list should be rejected")

	local ok_work =
		pcall(place_blueprint, "hut", { Square = { x = 0, y = 0, z = 0 } }, { { kind = "wood", amount = 1 } }, 0)
	assert.is_false(ok_work, "required_work < 1 should be rejected")
end

local function test_topology_mismatch_rejected()
	set_mode("colony")
	local hex_map = {}
	hex_map.topology = "hex"
	hex_map.cells = {}
	hex_map.cells[1] = { q = 0, r = 0, z = 0 }
	register_map("field", hex_map)
	set_active_map("field")

	local ok = pcall(place_blueprint, "hut", { Square = { x = 0, y = 0, z = 0 } }, { { kind = "wood", amount = 1 } }, 1)
	assert.is_false(ok, "Square cell on a hex map should be rejected")
end

local function test_full_build_completes_with_event()
	local agent, stockpile = setup_build_world()

	local site = place_blueprint("hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 2 } }, 2)
	deliver_to_site(agent, stockpile, site)

	for _ = 1, 10 do
		run_native_system("ConstructionSystem")
		if get_construction_state(site).state == "complete" then
			break
		end
	end

	local done = get_construction_state(site)
	assert.equals(done.state, "complete")
	assert.equals(done.building_type, "hut")

	update_event_buses()
	local events = poll_ecs_event("construction_completed")
	local found = false
	for _, event in ipairs(events) do
		if event.site_id == site and event.building_id == site and event.building_type == "hut" then
			found = true
		end
	end
	assert.is_true(found, "construction_completed event should carry site, building, and type")

	local building = get_component(site, "Building")
	assert.not_nil(building, "Site entity should carry Building after completion")
	assert.is_nil(get_component(site, "ConstructionSite"), "ConstructionSite should be replaced")

	local resources = get_component(stockpile, "Stockpile").resources
	assert.equals(resources.wood, 0, "Stockpile should be consumed exactly once on delivery")
end

local function test_cancel_refunds_and_removes_site()
	local agent, stockpile = setup_build_world()

	local site = place_blueprint("hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 2 } }, 5)
	local job_id = deliver_to_site(agent, stockpile, site)

	run_native_system("ConstructionSystem")
	assert.equals(get_construction_state(site).state, "in_progress")
	assert.equals(get_component(stockpile, "Stockpile").resources.wood, 0)

	assert.equals(true, cancel_construction(site), "Cancel should succeed before completion")
	assert.equals(get_component(stockpile, "Stockpile").resources.wood, 2, "Delivered materials should be refunded")
	assert.equals(get_job(job_id).state, "cancelled")
	assert.is_nil(get_component(site, "ConstructionSite"), "Ghost entity should be removed")

	local ok_state = pcall(get_construction_state, site)
	assert.is_false(ok_state, "State query on a cancelled site should error")
end

local function test_demolish_removes_completed_building()
	local agent, stockpile = setup_build_world()

	local site = place_blueprint("hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 2 } }, 1)
	deliver_to_site(agent, stockpile, site)
	for _ = 1, 10 do
		run_native_system("ConstructionSystem")
		if get_construction_state(site).state == "complete" then
			break
		end
	end
	assert.equals(get_construction_state(site).state, "complete")

	local ok_cancel = pcall(cancel_construction, site)
	assert.is_false(ok_cancel, "Cancel after completion should be rejected")

	assert.equals(true, demolish_building(site), "Demolish should remove the completed building")
	assert.is_nil(get_component(site, "Building"), "Building entity should be gone")
	assert.equals(get_component(stockpile, "Stockpile").resources.wood, 0, "Demolish refunds nothing")

	local ok_demolish = pcall(demolish_building, 99999)
	assert.is_false(ok_demolish, "Demolish of an unknown id should error")
end

local function test_mode_gate_outside_colony()
	set_mode("roguelike")

	local ok_place =
		pcall(place_blueprint, "hut", { Square = { x = 2, y = 0, z = 0 } }, { { kind = "wood", amount = 1 } }, 1)
	assert.is_false(ok_place, "place_blueprint outside colony mode should error")

	local ok_state = pcall(get_construction_state, 1)
	assert.is_false(ok_state, "get_construction_state outside colony mode should error")
end

return {
	test_place_blueprint_and_query_state = test_place_blueprint_and_query_state,
	test_blueprint_validation_rejects_bad_input = test_blueprint_validation_rejects_bad_input,
	test_topology_mismatch_rejected = test_topology_mismatch_rejected,
	test_full_build_completes_with_event = test_full_build_completes_with_event,
	test_cancel_refunds_and_removes_site = test_cancel_refunds_and_removes_site,
	test_demolish_removes_completed_building = test_demolish_removes_completed_building,
	test_mode_gate_outside_colony = test_mode_gate_outside_colony,
}
