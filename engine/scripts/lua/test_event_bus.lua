-- Event bus test: single and batch event polling

-- Single event: number
send_event("test_event", "42")
update_event_buses()
local events = poll_event("test_event")
print("Number event:", type(events[1]), events[1])
assert(events[1] ~= nil and type(events[1]) == "number" and events[1] == 42)

-- Single event: string
send_event("test_event", '"hello"')
send_event("test_event", '"world"')
update_event_buses()
local events = poll_event("test_event")
print("String events:", type(events[1]), events[1], type(events[2]), events[2])
assert(events[1] ~= nil and type(events[1]) == "string" and events[1] == "hello")

-- Single event: table/object
send_event("test_event", '{"foo":123,"bar":"baz"}')
update_event_buses()
local events = poll_event("test_event")
print("Table event:", type(events[1]), events[1])
assert(events[1] ~= nil and type(events[1]) == "table")
assert(events[1].foo == 123)
assert(events[1].bar == "baz")

print("Single-event polling passed!")

-- Batch event test: send multiple before update

send_event("test_event", "100")
send_event("test_event", '"world"')
send_event("test_event", '{"bar":456}')
update_event_buses()
local events = poll_event("test_event")
print("Batch events:", type(events[1]), events[1], type(events[2]), events[2], type(events[3]), events[3])
assert(events[1] ~= nil and type(events[1]) == "number" and events[1] == 100)
assert(events[2] ~= nil and type(events[2]) == "string" and events[2] == "world")
assert(events[3] ~= nil and type(events[3]) == "table" and events[3].bar == 456)

print("Batch event polling passed!")
print("All event bus tests passed!")
