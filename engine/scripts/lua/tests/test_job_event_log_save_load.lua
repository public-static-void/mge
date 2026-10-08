local assert = require("assert")
local tmp_path = require("helpers.tmp_path")

local function test_event_log_save_and_load()
	-- Reset the process-global event log so this test is isolated from
	-- whatever ran before it in the same runner process.
	job_events.reset_for_tests()
	set_mode("colony")
	local agent = spawn_entity()
	set_component(agent, "Agent", { entity_id = agent, skills = { TestJob = 1.0 } })
	local e1 = spawn_entity()
	assign_job(e1, "TestJob", { state = "pending", progress = 0.0, category = "test", assigned_to = agent })
	advance_job_state(e1)
	local events_before = job_events.get_log()
	assert.is_table(events_before, "job_events.get_log should return a table")
	assert.is_true(#events_before > 0, "Should have at least one event before save")

	-- Save the event log to a file
	local log_path = tmp_path.tmp_save_path("test_job_event_log")
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
