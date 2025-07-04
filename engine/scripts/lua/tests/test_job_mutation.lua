local assert = require("assert")

local function test_set_job_field()
	set_mode("colony")
	local eid = spawn_entity()
	assign_job(eid, "TestJob", { state = "pending", progress = 0.0, category = "test" })
	local jobs = list_jobs()
	assert.is_table(jobs, "list_jobs should return a table")
	assert.not_nil(jobs[1], "There should be at least one job")
	local job_id = jobs[1].id

	set_job_field(job_id, "state", "in_progress")
	local job = get_job(job_id)
	assert.equals(job.state, "in_progress", "Job state should be updated to in_progress")

	set_job_field(job_id, "progress", 0.5)
	job = get_job(job_id)
	assert.equals(job.progress, 0.5, "Job progress should be updated to 0.5")
end

local function test_update_job()
	set_mode("colony")
	local eid = spawn_entity()
	assign_job(eid, "TestJob", { state = "pending", progress = 0.0, category = "test" })
	local jobs = list_jobs()
	assert.is_table(jobs, "list_jobs should return a table")
	assert.not_nil(jobs[1], "There should be at least one job")
	local job_id = jobs[1].id

	update_job(job_id, { state = "complete", progress = 1.0, custom = "foo" })
	local job = get_job(job_id)
	assert.equals(job.state, "complete", "Job state should be updated to complete")
	assert.equals(job.progress, 1.0, "Job progress should be updated to 1.0")
	assert.equals(job.custom, "foo", "Job custom field should be updated to 'foo'")
end
