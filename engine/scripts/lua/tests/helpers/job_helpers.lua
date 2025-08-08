local M = {}

function M.dummy_job_handler(job, progress)
	if job.state == "pending" then
		job.state = "in_progress"
	elseif job.state == "in_progress" then
		job.state = "complete"
	end
	return job
end

function M.noop_job_handler(job, progress)
	return job
end

return M
