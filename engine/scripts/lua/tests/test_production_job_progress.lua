local assert = require("assert")

local function test_production_job_progress_and_state()
	set_mode("colony")
	local e = spawn_entity()
	set_component(e, "ProductionJob", { state = "pending", progress = 0, recipe = "wood_plank" })

	local progress = get_production_job_progress(e)
	assert.equals(0, progress, "Initial production job progress should be 0")
	local state = get_production_job_state(e)
	assert.equals("pending", state, "Initial production job state should be 'pending'")

	set_production_job_state(e, "in_progress")
	set_production_job_progress(e, 2)
	local new_progress = get_production_job_progress(e)
	assert.equals(2, new_progress, "Production job progress should be 2 after set")
	local new_state = get_production_job_state(e)
	assert.equals("in_progress", new_state, "Production job state should be 'in_progress' after set")
end

return {
	test_production_job_progress_and_state = test_production_job_progress_and_state,
}
