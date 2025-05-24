local assert = require("assert")

local function test_single_event_polling()
	-- Single event: number
	send_event("test_event", "42")
	update_event_buses()
	local events = poll_event("test_event")
	assert.not_nil(events[1])
	assert.equals(type(events[1]), "number")
	assert.equals(events[1], 42)

	-- Single event: string
	send_event("test_event", '"hello"')
	send_event("test_event", '"world"')
	update_event_buses()
	events = poll_event("test_event")
	assert.not_nil(events[1])
	assert.equals(type(events[1]), "string")
	assert.equals(events[1], "hello")

	-- Single event: table/object
	send_event("test_event", '{"foo":123,"bar":"baz"}')
	update_event_buses()
	events = poll_event("test_event")
	assert.not_nil(events[1])
	assert.equals(type(events[1]), "table")
	assert.equals(events[1].foo, 123)
	assert.equals(events[1].bar, "baz")
end

local function test_batch_event_polling()
	-- Batch event test: send multiple before update
	send_event("test_event", "100")
	send_event("test_event", '"world"')
	send_event("test_event", '{"bar":456}')
	update_event_buses()
	local events = poll_event("test_event")
	assert.not_nil(events[1])
	assert.equals(type(events[1]), "number")
	assert.equals(events[1], 100)
	assert.not_nil(events[2])
	assert.equals(type(events[2]), "string")
	assert.equals(events[2], "world")
	assert.not_nil(events[3])
	assert.equals(type(events[3]), "table")
	assert.equals(events[3].bar, 456)
end

return {
	test_single_event_polling = test_single_event_polling,
	test_batch_event_polling = test_batch_event_polling,
}
