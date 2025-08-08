local assert = require("assert")
local job_helpers = require("helpers.job_helpers")

local function test_get_job_board()
	set_mode("colony")
	register_job_type("JobA", job_helpers.dummy_job_handler)
	register_job_type("JobB", job_helpers.dummy_job_handler)

	local eid1 = spawn_entity()
	local eid2 = spawn_entity()
	local agent1 = spawn_entity()
	local agent2 = spawn_entity()
	-- Add Agent components with skills matching the job types
	set_component(agent1, "Agent", { entity_id = agent1, skills = { JobA = 1.0 } })
	set_component(agent2, "Agent", { entity_id = agent2, skills = { JobB = 1.0 } })

	assign_job(eid1, "JobA", { state = "pending", priority = 5, category = "test" })
	assign_job(eid2, "JobB", { state = "pending", priority = 10, category = "test" })

	local jobs = get_job_board()
	assert.is_table(jobs, "get_job_board should return a table")
	assert.equals(2, #jobs, "Should have two jobs on the board")

	-- Should be sorted by priority descending by default
	assert.equals(eid2, jobs[1].eid, "Highest priority first")
	assert.equals(eid1, jobs[2].eid, "Lower priority second")
	assert.equals(10, jobs[1].priority)
	assert.equals(5, jobs[2].priority)
	assert.equals("pending", jobs[1].state)
end

local function test_job_board_policy_and_priority()
	set_mode("colony")

	register_job_type("JobA", job_helpers.noop_job_handler)
	register_job_type("JobB", job_helpers.noop_job_handler)
	register_job_type("JobC", job_helpers.noop_job_handler)

	local eid1 = spawn_entity()
	local eid2 = spawn_entity()
	local eid3 = spawn_entity()
	local agent1 = spawn_entity()
	local agent2 = spawn_entity()
	local agent3 = spawn_entity()
	-- Set Agent components with skills for each job type
	set_component(agent1, "Agent", { entity_id = agent1, skills = { JobA = 1.0 } })
	set_component(agent2, "Agent", { entity_id = agent2, skills = { JobB = 1.0 } })
	set_component(agent3, "Agent", { entity_id = agent3, skills = { JobC = 1.0 } })

	assign_job(eid1, "JobA", { state = "pending", priority = 5, category = "test" })
	assign_job(eid2, "JobB", { state = "pending", priority = 10, category = "test" })
	assign_job(eid3, "JobC", { state = "pending", priority = 1, category = "test" })

	local jobs = get_job_board()
	assert.is_table(jobs, "get_job_board should return a table")
	assert.equals(3, #jobs, "Should have three jobs on the board")

	assert.equals(eid2, jobs[1].eid)
	assert.equals(eid1, jobs[2].eid)
	assert.equals(eid3, jobs[3].eid)

	-- Change to FIFO
	set_job_board_policy("fifo")
	assert.equals("fifo", get_job_board_policy())
	jobs = get_job_board()
	assert.is_table(jobs)
	assert.equals(eid1, jobs[1].eid)
	assert.equals(eid2, jobs[2].eid)
	assert.equals(eid3, jobs[3].eid)

	-- Change to LIFO
	set_job_board_policy("lifo")
	assert.equals("lifo", get_job_board_policy())
	jobs = get_job_board()
	assert.is_table(jobs)
	assert.equals(eid3, jobs[1].eid)
	assert.equals(eid2, jobs[2].eid)
	assert.equals(eid1, jobs[3].eid)

	-- Test get/set job priority
	assert.equals(5, get_job_priority(eid1))
	set_job_priority(eid1, 42)
	assert.equals(42, get_job_priority(eid1))
end

return {
	test_get_job_board = test_get_job_board,
	test_job_board_policy_and_priority = test_job_board_policy_and_priority,
}
