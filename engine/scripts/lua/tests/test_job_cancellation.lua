local assert = require("assert")

local function test_cancel_job_marks_cancelled_and_filters_from_active()
	set_mode("colony")
	local eid = spawn_entity()
	assign_job(eid, "TestJob", { state = "pending", category = "test" })
	local jobs = list_jobs()
	assert.is_table(jobs)
	assert.equals(1, #jobs)
	local job_id = jobs[1].id

	cancel_job(job_id)
	local job = get_job(job_id)
	assert.is_true(job.cancelled, "Job should be marked as cancelled")

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
