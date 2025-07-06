local assert = require("assert")

-- Test querying the job event log: all, by type, since
local function test_job_event_log_querying()
	init_job_event_logger()
	set_mode("colony")
	local agent1 = spawn_entity()
	local agent2 = spawn_entity()
	set_component(agent1, "Agent", { entity_id = agent1, skills = { DigTunnel = 1.0 } })
	set_component(agent2, "Agent", { entity_id = agent2, skills = { BuildWall = 1.0 } })

	local eid1 = spawn_entity()
	local eid2 = spawn_entity()
	assign_job(eid1, "DigTunnel", { state = "pending", priority = 5, category = "test" })
	assign_job(eid2, "BuildWall", { state = "pending", priority = 10, category = "test" })

	for _ = 1, 10 do
		tick()
	end

	local events = job_events.get_log()
	assert.is_table(events, "get_log should return a table")
	-- At least one job_assigned or job_completed event should be present.
	local found_assigned, found_completed = false, false
	for _, e in ipairs(events) do
		if e.event_type == "job_assigned" then
			found_assigned = true
		end
		if e.event_type == "job_completed" then
			found_completed = true
		end
	end
	assert.is_true(found_assigned or found_completed, "Expected at least one job_assigned or job_completed event")

	local assigned_events = job_events.get_by_type("job_assigned")
	assert.is_table(assigned_events, "get_by_type should return a table")
	for _, e in ipairs(assigned_events) do
		assert.equals("job_assigned", e.event_type)
	end

	local now = os.time() * 1000
	local job3 = spawn_entity()
	assign_job(job3, "TestJob", { state = "pending", category = "test" })
	local agent3 = spawn_entity()
	set_component(agent3, "Agent", { entity_id = agent3, skills = { TestJob = 1.0 } })
	for _ = 1, 5 do
		tick()
	end
	local events_since = job_events.get_since(now)
	assert.is_table(events_since, "get_since should return a table")
	local found_recent = false
	for _, e in ipairs(events_since) do
		if e.timestamp >= now then
			found_recent = true
			break
		end
	end
	assert.is_true(found_recent, "Expected at least one event since timestamp")
end

-- Test polling the job event bus for a specific event type
local function test_job_event_bus_polling()
	init_job_event_logger()
	set_mode("colony")
	local agent = spawn_entity()
	set_component(agent, "Agent", { entity_id = agent, skills = { DigTunnel = 1.0 } })
	local eid = spawn_entity()
	assign_job(eid, "DigTunnel", { state = "pending", category = "test" })
	for _ = 1, 5 do
		tick()
	end

	local events = job_events.poll_bus("job_assigned")
	assert.is_table(events, "poll_bus should return a table")
	for _, e in ipairs(events) do
		assert.equals("job_assigned", e.event_type)
	end
end

-- Test subscribing to the job event bus and receiving callbacks, then unsubscribing
local function test_job_event_bus_subscription()
	init_job_event_logger()
	set_mode("colony")
	local agent = spawn_entity()
	set_component(agent, "Agent", { entity_id = agent, skills = { DigTunnel = 1.0 } })
	local eid = spawn_entity()
	local received = {}

	local function on_job_completed(event)
		table.insert(received, event)
	end

	local sub_id = job_events.subscribe_bus("job_completed", on_job_completed)
	assign_job(eid, "DigTunnel", { state = "pending", category = "test" })
	for _ = 1, 10 do
		tick()
		job_events.deliver_callbacks()
		if #received > 0 then
			break
		end
	end

	assert.is_true(#received > 0, "Should have received job_completed event")
	assert.equals("job_completed", received[1].event_type)

	-- Unsubscribe and ensure no more events are received
	job_events.unsubscribe_bus("job_completed", sub_id)
	for i = 1, #received do
		received[i] = nil
	end

	local job2 = spawn_entity()
	local agent2 = spawn_entity()
	set_component(agent2, "Agent", { entity_id = agent2, skills = { DigTunnel = 1.0 } })
	assign_job(job2, "DigTunnel", { state = "pending", category = "test" })
	for _ = 1, 10 do
		tick()
		job_events.deliver_callbacks()
	end
	assert.equals(0, #received, "Should not receive events after unsubscribe")
end

return {
	test_job_event_log_querying = test_job_event_log_querying,
	test_job_event_bus_polling = test_job_event_bus_polling,
	test_job_event_bus_subscription = test_job_event_bus_subscription,
}
