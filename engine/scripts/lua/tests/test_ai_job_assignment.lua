local assert = require("assert")
local ai_helpers = require("helpers.ai_job_helpers")
local job_helpers = require("helpers.job_helpers") -- if needed

local function test_assign_jobs_basic()
	init_job_event_logger()
	local agent = ai_helpers.spawn_agent_with_skills({ TestJob = 1.0 })

	local job_entity = spawn_entity()
	assign_job(job_entity, "TestJob", { state = "pending", category = "test", assigned_to = nil })

	local jobs_before = list_jobs()
	local found_unassigned = false
	for _, j in ipairs(jobs_before) do
		if j.id == job_entity and (not j.assigned_to or j.assigned_to == 0) then
			found_unassigned = true
		end
	end
	assert.is_true(found_unassigned, "Job should initially be unassigned")

	ai_assign_jobs(agent, {})

	local jobs_after = list_jobs()
	local found_assigned = false
	for _, j in ipairs(jobs_after) do
		if j.id == job_entity and j.assigned_to == agent then
			found_assigned = true
		end
	end
	assert.is_true(found_assigned, "Job should be assigned to the agent by AI logic")
end

local function test_assign_jobs_multiple_agents()
	local agent1 = ai_helpers.spawn_agent_with_skills({ TestJob = 1.0 })
	local agent2 = ai_helpers.spawn_agent_with_skills({ TestJob = 1.0 })

	local job1 = spawn_entity()
	assign_job(job1, "TestJob", { state = "pending", category = "test" })

	local job2 = spawn_entity()
	assign_job(job2, "TestJob", { state = "pending", category = "test" })

	ai_assign_jobs(agent1, {})
	ai_assign_jobs(agent2, {})

	local jobs = list_jobs()
	local assigned_jobs = 0
	local assigned_counts = {}
	for _, j in ipairs(jobs) do
		if j.state == "pending" or j.state == "in_progress" then
			assigned_jobs = assigned_jobs + 1
			assigned_counts[j.assigned_to] = (assigned_counts[j.assigned_to] or 0) + 1
		end
	end

	assert.equals(2, assigned_jobs, "Both jobs should be assigned")

	local count1 = assigned_counts[agent1] or 0
	local count2 = assigned_counts[agent2] or 0
	local diff = math.abs(count1 - count2)
	assert.is_true(diff <= 1, "Jobs should be fairly assigned among agents")
end

local function test_assign_jobs_no_jobs()
	local agent = ai_helpers.spawn_agent_with_skills({ TestJob = 1.0 })

	ai_assign_jobs(agent, {})

	local jobs = list_jobs()
	assert.equals(0, #jobs, "No jobs should exist in world")
end

return {
	test_assign_jobs_basic = test_assign_jobs_basic,
	test_assign_jobs_multiple_agents = test_assign_jobs_multiple_agents,
	test_assign_jobs_no_jobs = test_assign_jobs_no_jobs,
}
