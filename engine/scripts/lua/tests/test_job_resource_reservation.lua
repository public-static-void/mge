local assert = require("assert")

local function test_job_resource_reservation_query()
	set_mode("colony")
	local e = spawn_entity()
	local agent_id = spawn_entity()
	-- Add Agent component with skill for "Build" jobs
	set_component(agent_id, "Agent", { entity_id = agent_id, skills = { Build = 1.0 } })
	set_component(e, "Job", {
		job_type = "Build",
		category = "construction",
		state = "pending",
		assigned_to = agent_id,
		reserved_resources = {
			{ kind = "wood", amount = 3 },
			{ kind = "stone", amount = 1 },
		},
	})

	local res = get_job_resource_reservations(e)
	assert.not_nil(res, "Should return a table for reserved resources")
	assert.equals(res[1].kind, "wood")
	assert.equals(res[1].amount, 3)
	assert.equals(res[2].kind, "stone")
	assert.equals(res[2].amount, 1)

	-- Should return nil if no reserved_resources field
	set_component(
		e,
		"Job",
		{ job_type = "Build", category = "construction", state = "pending", assigned_to = agent_id }
	)
	local none = get_job_resource_reservations(e)
	assert.is_nil(none, "Should return nil if no reserved_resources")
end

local function test_job_resource_reservation_mutation()
	set_mode("colony")
	local stockpile = spawn_entity()
	set_component(stockpile, "Stockpile", { resources = { wood = 10 } })

	local e = spawn_entity()
	local agent_id = spawn_entity()
	-- Add Agent component with skill for "Build" jobs
	set_component(agent_id, "Agent", { entity_id = agent_id, skills = { Build = 1.0 } })
	set_component(e, "Job", {
		job_type = "Build",
		category = "construction",
		state = "pending",
		assigned_to = agent_id,
		resource_requirements = {
			{ kind = "wood", amount = 3 },
		},
	})

	assert.is_nil(get_job_resource_reservations(e))

	local reserved = reserve_job_resources(e)

	local res = get_job_resource_reservations(e)
	assert.equals(true, reserved, "Should successfully reserve resources")
	assert.not_nil(res)
	assert.equals(res[1].kind, "wood")
	assert.equals(res[1].amount, 3)

	release_job_resource_reservations(e)
	assert.is_nil(get_job_resource_reservations(e))
end

return {
	test_job_resource_reservation_query = test_job_resource_reservation_query,
	test_job_resource_reservation_mutation = test_job_resource_reservation_mutation,
}
