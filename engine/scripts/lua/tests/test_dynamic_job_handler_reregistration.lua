local assert = require("assert")

local function test_dynamic_job_handler_reregistration()
	init_job_event_logger()
	set_mode("colony")
	local eid = spawn_entity()
	local state = { which = nil }

	register_job_type("LuaJob", function(job, progress)
		state.which = "first"
		job.state = "complete"
		return job
	end)
	-- Overwrite handler with a new one
	register_job_type("LuaJob", function(job, progress)
		state.which = "second"
		job.state = "complete"
		return job
	end)

	assign_job(eid, "LuaJob", { state = "pending", category = "testing" })
	run_native_system("JobSystem")
	tick()
	local job = get_component(eid, "Job")
	assert.equals(job.state, "complete", "Job should be complete")
	assert.equals(state.which, "second", "Second handler should be used")
end

return {
	test_dynamic_job_handler_reregistration = test_dynamic_job_handler_reregistration,
}
