local assert = require("assert")

local function test_list_jobs()
	init_job_event_logger()
	set_mode("colony")
	local e1 = spawn_entity()
	local e2 = spawn_entity()
	assign_job(e1, "TestJob", { state = "pending", category = "test" })
	assign_job(e2, "TestJob", { state = "in_progress", category = "test" })
	local jobs = list_jobs()
	assert.is_table(jobs, "list_jobs should return a table")
	assert.equals(2, #jobs, "Should have two jobs")
	local states = { jobs[1].state, jobs[2].state }
	table.sort(states)
	assert.equals("in_progress", states[1], "First job state should be in_progress")
	assert.equals("pending", states[2], "Second job state should be pending")
end

local function test_get_job_by_id()
	init_job_event_logger()
	set_mode("colony")
	local e = spawn_entity()
	assign_job(e, "TestJob", { state = "pending", category = "test" })
	local jobs = list_jobs()
	assert.is_table(jobs, "list_jobs should return a table")
	assert.is_true(#jobs >= 1, "Should have at least one job")
	local job_id = jobs[1].id
	local job = get_job(job_id)
	assert.is_table(job, "get_job should return a table")
	assert.equals("pending", job.state, "Job state should be pending")
	assert.equals("TestJob", job.job_type, "Job type should be TestJob")
end

local function test_find_jobs_by_state_and_type()
	init_job_event_logger()
	set_mode("colony")
	local e1 = spawn_entity()
	local e2 = spawn_entity()
	assign_job(e1, "TestJob", { state = "pending", category = "test" })
	assign_job(e2, "OtherJob", { state = "in_progress", category = "test" })
	local pending_jobs = find_jobs({ state = "pending" })
	assert.equals(1, #pending_jobs, "Should find one pending job")
	assert.equals("TestJob", pending_jobs[1].job_type, "Pending job should be TestJob")
	local other_jobs = find_jobs({ job_type = "OtherJob" })
	assert.equals(1, #other_jobs, "Should find one job of type OtherJob")
	assert.equals("in_progress", other_jobs[1].state, "OtherJob should be in_progress")
end

return {
	test_list_jobs = test_list_jobs,
	test_get_job_by_id = test_get_job_by_id,
	test_find_jobs_by_state_and_type = test_find_jobs_by_state_and_type,
}
