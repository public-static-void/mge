local luaunit = require("luaunit")

TestEventBus = {}

function TestEventBus:test_single_event_polling()
	-- Single event: number
	send_event("test_event", "42")
	update_event_buses()
	local events = poll_event("test_event")
	luaunit.assertNotNil(events[1])
	luaunit.assertEquals(type(events[1]), "number")
	luaunit.assertEquals(events[1], 42)

	-- Single event: string
	send_event("test_event", '"hello"')
	send_event("test_event", '"world"')
	update_event_buses()
	local events = poll_event("test_event")
	luaunit.assertNotNil(events[1])
	luaunit.assertEquals(type(events[1]), "string")
	luaunit.assertEquals(events[1], "hello")

	-- Single event: table/object
	send_event("test_event", '{"foo":123,"bar":"baz"}')
	update_event_buses()
	local events = poll_event("test_event")
	luaunit.assertNotNil(events[1])
	luaunit.assertEquals(type(events[1]), "table")
	luaunit.assertEquals(events[1].foo, 123)
	luaunit.assertEquals(events[1].bar, "baz")
end

function TestEventBus:test_batch_event_polling()
	-- Batch event test: send multiple before update
	send_event("test_event", "100")
	send_event("test_event", '"world"')
	send_event("test_event", '{"bar":456}')
	update_event_buses()
	local events = poll_event("test_event")
	luaunit.assertNotNil(events[1])
	luaunit.assertEquals(type(events[1]), "number")
	luaunit.assertEquals(events[1], 100)
	luaunit.assertNotNil(events[2])
	luaunit.assertEquals(type(events[2]), "string")
	luaunit.assertEquals(events[2], "world")
	luaunit.assertNotNil(events[3])
	luaunit.assertEquals(type(events[3]), "table")
	luaunit.assertEquals(events[3].bar, 456)
end

os.exit(luaunit.LuaUnit.run())
