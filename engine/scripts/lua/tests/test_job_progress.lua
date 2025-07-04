local assert = require("assert")

local function test_advance_job_progress()
	init_job_event_logger()
	local eid = spawn_entity()
	assign_job(eid, "TestJob", { state = "pending", progress = 0.0, category = "test" })

	local jobs = list_jobs()
	assert.equals(1, #jobs)
	local job_id = jobs[1].id

	local job = get_job(job_id)
	assert.equals("pending", job.state)
	assert.equals(0.0, job.progress)

	-- Advance the job once
	advance_job_state(job_id)

	job = get_job(job_id)
	assert.is_true(job.state == "in_progress" or job.state == "pending")
	assert.is_true(job.progress > 0.0)

	-- Advance multiple times to complete the job
	for _ = 1, 10 do
		advance_job_state(job_id)
	end

	job = get_job(job_id)
	assert.equals("complete", job.state)
	assert.is_true(job.progress >= 3.0)

	-- Advancing a completed job should not change state
	local prev_progress = job.progress
	advance_job_state(job_id)
	job = get_job(job_id)
	assert.equals("complete", job.state)
	assert.equals(prev_progress, job.progress)
end

return {
	test_advance_job_progress = test_advance_job_progress,
}
