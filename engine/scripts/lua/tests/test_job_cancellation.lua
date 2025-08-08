local assert = require("assert")

local function test_cancel_job_marks_cancelled_and_filters_from_active()
	init_job_event_logger()
	set_mode("colony")
	local eid = spawn_entity()
	local agent_id = spawn_entity()
	-- Add Agent component with skill for "TestJob"
	set_component(agent_id, "Agent", { entity_id = agent_id, skills = { TestJob = 1.0 } })

	assign_job(eid, "TestJob", { state = "pending", category = "test", assigned_to = agent_id })
	local jobs = list_jobs()
	assert.is_table(jobs)
	assert.equals(1, #jobs)
	local job_id = jobs[1].id

	cancel_job(job_id)

	-- Allow job system to process cancellations
	for _ = 1, 5 do
		run_native_system("JobSystem")
		tick()
	end

	local job = get_job(job_id)
	assert.equals("cancelled", job.state, "Job state should be 'cancelled'")

	for _ = 1, 3 do
		tick()
	end

	local jobs_after = list_jobs()
	for _, job in ipairs(jobs_after) do
		assert.is_true(job.id ~= job_id, "Cancelled job should not be in active jobs")
	end

	local found = false
	for _, job in ipairs(list_jobs({ include_terminal = true })) do
		if job.id == job_id then
			found = true
		end
	end
	assert.is_true(found, "Cancelled job should be available for introspection")
end

return {
	test_cancel_job_marks_cancelled_and_filters_from_active = test_cancel_job_marks_cancelled_and_filters_from_active,
}
