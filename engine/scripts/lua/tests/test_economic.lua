local assert = require("assert")

function test_economic_helpers()
	local eid = spawn_entity()
	set_component(eid, "Stockpile", { resources = { wood = 5 } })
	set_component(eid, "ProductionJob", {
		recipe = "wood_plank",
		progress = 0,
		state = "pending",
	})

	-- Test helper: get_stockpile_resources
	local resources = get_stockpile_resources(eid)
	assert.not_nil(resources, "get_stockpile_resources should return a table")
	assert.equals(resources.wood, 5, "Stockpile should have 5 wood")

	-- Test helper: get_production_job
	local job = get_production_job(eid)
	assert.not_nil(job, "get_production_job should return a table")
	assert.equals(job.recipe, "wood_plank", "Job recipe should be wood_plank")
	assert.equals(job.state, "pending", "Job state should be pending")

	-- Remove and test nil
	remove_component(eid, "Stockpile")
	assert.is_nil(get_stockpile_resources(eid), "Should return nil after removing stockpile")

	remove_component(eid, "ProductionJob")
	assert.is_nil(get_production_job(eid), "Should return nil after removing job")
end

function test_enqueue_production_job()
	local eid = spawn_entity()

	-- First enqueue should return true
	local result = enqueue_production_job(eid, "wood_plank", 1, 2)
	assert.equals(true, result, "First enqueue should return true")

	-- Verify the component was created with correct fields
	local job = get_production_queue(eid)
	assert.not_nil(job, "ProductionJob should exist after enqueue")
	assert.equals(job.recipe, "wood_plank", "Recipe should be wood_plank")
	assert.equals(job.priority, 1, "Priority should be 1")
	assert.equals(job.batch_size, 2, "Batch size should be 2")
	assert.equals(job.progress, 0, "Progress should be 0")
	assert.equals(job.state, "pending", "State should be pending")

	-- Second enqueue on same entity should return false (no-op)
	local result2 = enqueue_production_job(eid, "stone_bricks", 0, 1)
	assert.equals(false, result2, "Second enqueue should return false (already has job)")
end

function test_get_production_queue()
	local eid = spawn_entity()

	-- No job yet, should return nil
	local no_job = get_production_queue(eid)
	assert.is_nil(no_job, "get_production_queue should return nil when no job exists")

	-- Enqueue a job and verify
	enqueue_production_job(eid, "wood_plank", 5, 1)
	local job = get_production_queue(eid)
	assert.not_nil(job, "get_production_queue should return a table after enqueue")
	assert.equals(job.recipe, "wood_plank", "Recipe should be wood_plank")
	assert.equals(job.priority, 5, "Priority should be 5")

	-- Non-existent entity should return nil
	local no_entity = get_production_queue(99999)
	assert.is_nil(no_entity, "get_production_queue should return nil for non-existent entity")
end

function test_get_completed_production_jobs()
	local eid = spawn_entity()
	set_component(eid, "Stockpile", { resources = { wood = 10 } })

	-- No completions yet
	local empty = get_completed_production_jobs(eid)
	assert.equals(0, #empty, "Should return empty array before any completions")

	-- Enqueue a production job
	enqueue_production_job(eid, "wood_plank", 0, 1)

	-- Run economic system until completion
	run_system("EconomicSystem")
	run_system("EconomicSystem")

	-- Update event buses so events are readable
	update_event_buses()

	-- Check completions
	local completions = get_completed_production_jobs(eid)
	assert.equals(1, #completions, "Should have 1 completion event")
	assert.equals(completions[1].recipe, "wood_plank", "Recipe should be wood_plank")
	assert.equals(completions[1].batch_count, 1, "Batch count should be 1")
	assert.not_nil(completions[1].outputs, "Should have outputs array")
end

function test_get_completed_production_jobs_empty()
	local eid = spawn_entity()

	-- No completions before any job exists
	local result = get_completed_production_jobs(eid)
	assert.equals(0, #result, "Should return empty array when no completions exist")
end

return {
	test_economic_helpers = test_economic_helpers,
	test_enqueue_production_job = test_enqueue_production_job,
	test_get_production_queue = test_get_production_queue,
	test_get_completed_production_jobs = test_get_completed_production_jobs,
	test_get_completed_production_jobs_empty = test_get_completed_production_jobs_empty,
}
