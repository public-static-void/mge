local eid = spawn_entity()
assign_job(eid, "test_job", { should_fail = false })
local found = false
for i = 1, 12 do
	run_native_system("JobSystem")
	update_event_buses()
	local events = poll_ecs_event("job_completed") -- use poll_ecs_event if available!
	if #events > 0 then
		for _, event in ipairs(events) do
			if event.entity == eid then
				found = true
				print("Job completed event received for entity", eid)
			end
		end
		if found then
			break
		end
	end
end
assert(found, "No job_completed events for this entity")
