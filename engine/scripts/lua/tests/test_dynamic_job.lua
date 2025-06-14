local assert = require("assert")

local function test_dynamic_job_type_registration()
	set_mode("colony")
	local eid = spawn_entity()
	register_job_type("LuaJob", function(job, progress)
		if job.status == "pending" then
			job.status = "in_progress"
		elseif job.status == "in_progress" then
			job.progress = (job.progress or 0) + 1
			if job.progress >= 2 then
				job.status = "complete"
			end
		end
		return job
	end)
	assign_job(eid, "LuaJob", { category = "testing" })
	for i = 1, 4 do
		run_native_system("JobSystem")
	end
	local job = get_component(eid, "Job")
	assert.equals(job.status, "complete", "Job should be complete")
end

return {
	test_dynamic_job_type_registration = test_dynamic_job_type_registration,
}
