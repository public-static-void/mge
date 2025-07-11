local assert = require("assert")

local function test_event_log_save_and_load()
	init_job_event_logger()
	set_mode("colony")
	local e1 = spawn_entity()
	assign_job(e1, "TestJob", { state = "pending", category = "test" })
	advance_job_state(e1)
	local events_before = job_events.get_log()
	assert.is_table(events_before, "job_events.get_log should return a table")
	assert.is_true(#events_before > 0, "Should have at least one event before save")

	-- Save the event log to a file
	local log_path = "test_job_event_log.json"
	job_events.save(log_path)

	-- Clear the event log (simulate fresh session)
	job_events.clear()
	local events_cleared = job_events.get_log()
	assert.equals(0, #events_cleared, "Event log should be empty after re-init")

	-- Load the event log from file
	job_events.load(log_path)
	local events_loaded = job_events.get_log()
	assert.equals(#events_before, #events_loaded, "Loaded event log should have same length as before")

	-- Replay the event log (should not error)
	job_events.replay()
end

return {
	test_event_log_save_and_load = test_event_log_save_and_load,
}
