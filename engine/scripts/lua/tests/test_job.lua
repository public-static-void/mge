local assert = require("assert")

local function test_job_completion_event()
	local eid = spawn_entity()
	assign_job(eid, "test_job", { should_fail = false })
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
