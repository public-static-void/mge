local assert = require("assert")

local function test_dynamic_job_type_registration()
	init_job_event_logger()
	set_mode("colony")
	local eid = spawn_entity()
	register_job_type("LuaJob", function(job, progress)
		if job.state == "pending" then
			job.state = "in_progress"
		elseif job.state == "in_progress" then
			job.progress = (job.progress or 0) + 1
			if job.progress >= 2 then
				job.state = "complete"
			end
		end
		return job
	end)
	assign_job(eid, "LuaJob", { category = "testing", state = "pending" })
	for i = 1, 4 do
		run_native_system("JobSystem")
	end
	local job = get_component(eid, "Job")
	assert.equals(job.state, "complete", "Job should be complete")
end

return {
	test_dynamic_job_type_registration = test_dynamic_job_type_registration,
}
