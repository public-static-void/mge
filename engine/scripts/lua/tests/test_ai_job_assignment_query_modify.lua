local assert = require("assert")
local ai_helpers = require("helpers.ai_job_helpers")
local job_helpers = require("helpers.job_helpers")

local function test_query_ai_jobs_empty()
	init_job_event_logger()
	local agent = ai_helpers.spawn_agent_with_skills({ test = 1.0 }, { "test" })

	-- Query AI assignments before any jobs assigned, should return empty table
	local assigned_jobs = ai_query_jobs(agent)
	assert.is_table(assigned_jobs)
	assert.equals(0, #assigned_jobs)
end

local function test_query_ai_jobs_after_assignment()
	init_job_event_logger()
	local agent = ai_helpers.spawn_agent_with_skills({ test = 1.0 }, { "test" })

	-- Spawn and assign job manually
	local job_entity = spawn_entity()
	assign_job(job_entity, "TestJob", { state = "pending", category = "test" })

	-- Add job to job board for AI assignment
	add_job_to_job_board(job_entity)

	-- Run assignment logic
	ai_assign_jobs(agent, {})

	-- Query AI jobs for agent returns at least one job
	local assigned_jobs = ai_query_jobs(agent)

	assert.is_table(assigned_jobs)
	local found = false
	for _, job in ipairs(assigned_jobs) do
		if tonumber(job.id) == tonumber(job_entity) then
			found = true
			break
		end
	end
	assert.is_true(found, "Assigned job should be returned in AI job query")
end

local function test_modify_ai_job_assignment_valid()
	init_job_event_logger()
	local agent = ai_helpers.spawn_agent_with_skills({ test = 1.0 }, { "test" })

	-- Spawn and assign job via AI logic
	local job_entity = spawn_entity()
	assign_job(job_entity, "TestJob", { state = "pending", category = "test" })

	-- Add job to job board
	add_job_to_job_board(job_entity)
	ai_assign_jobs(agent, {})

	-- Modify job assignment fields: reset assigned_to to nil to force unassign
	local success = ai_modify_job_assignment(job_entity, { assigned_to = nil })
	assert.is_true(success, "Modification call should succeed")

	-- Query jobs to confirm modification
	local assigned_jobs = ai_query_jobs(agent)
	local found = false
	for _, job in ipairs(assigned_jobs) do
		if tonumber(job.id) == tonumber(job_entity) then
			found = true
			break
		end
	end
	assert.is_false(found, "Job should no longer be assigned after modification")
end

local function test_modify_ai_job_assignment_invalid()
	init_job_event_logger()
	local invalid_job_id = 999999

	local ok, err = pcall(function()
		ai_modify_job_assignment(invalid_job_id, { assigned_to = 0 })
	end)

	assert.is_false(ok, "Modification on invalid job id should error")
	assert.is_true(string.match(tostring(err), "No job with id") ~= nil, "Error message should mention missing job")
end

return {
	test_query_ai_jobs_empty = test_query_ai_jobs_empty,
	test_query_ai_jobs_after_assignment = test_query_ai_jobs_after_assignment,
	test_modify_ai_job_assignment_valid = test_modify_ai_job_assignment_valid,
	test_modify_ai_job_assignment_invalid = test_modify_ai_job_assignment_invalid,
}
