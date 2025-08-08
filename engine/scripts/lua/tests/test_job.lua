local assert = require("assert")

local function test_job_completion_event()
	init_job_event_logger()
	local eid = spawn_entity()
	local agent_id = spawn_entity()
	-- Add Agent component with skill for "test_job"
	set_component(agent_id, "Agent", { entity_id = agent_id, skills = { test_job = 1.0 } })

	assign_job(
		eid,
		"test_job",
		{ should_fail = false, category = "testing", state = "pending", assigned_to = agent_id }
	)
	local found = false
	for i = 1, 12 do
		run_native_system("JobSystem")
		update_event_buses()
		local events = poll_ecs_event("job_completed")
		if #events > 0 then
			for _, event in ipairs(events) do
				if event.entity == eid then
					found = true
				end
			end
			if found then
				break
			end
		end
	end
	assert.is_true(found, "No job_completed events for this entity")
end

return {
	test_job_completion_event = test_job_completion_event,
}
