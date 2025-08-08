local M = {}

function M.spawn_agent_with_skills(skills, specializations)
	local eid = spawn_entity()
	set_component(eid, "Agent", {
		entity_id = eid,
		skills = skills or {},
		specializations = specializations or nil,
		state = "idle",
		current_job = nil,
		job_queue = nil,
	})
	return eid
end

function M.count_active_jobs(predicate)
	local jobs = list_jobs()
	local count = 0
	for _, job in ipairs(jobs) do
		if not predicate or predicate(job) then
			count = count + 1
		end
	end
	return count
end

return M
