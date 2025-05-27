local assert = require("assert")

function test_economic_helpers()
	local eid = spawn_entity()
	set_component(eid, "Stockpile", { resources = { wood = 5 } })
	set_component(eid, "ProductionJob", {
		recipe = "wood_plank",
		progress = 0,
		status = "pending",
	})

	-- Test helper: get_stockpile_resources
	local resources = get_stockpile_resources(eid)
	assert.not_nil(resources, "get_stockpile_resources should return a table")
	assert.equals(resources.wood, 5, "Stockpile should have 5 wood")

	-- Test helper: get_production_job
	local job = get_production_job(eid)
	assert.not_nil(job, "get_production_job should return a table")
	assert.equals(job.recipe, "wood_plank", "Job recipe should be wood_plank")
	assert.equals(job.status, "pending", "Job status should be pending")

	-- Remove and test nil
	remove_component(eid, "Stockpile")
	assert.is_nil(get_stockpile_resources(eid), "Should return nil after removing stockpile")

	remove_component(eid, "ProductionJob")
	assert.is_nil(get_production_job(eid), "Should return nil after removing job")
end

return { test_economic_helpers = test_economic_helpers }
