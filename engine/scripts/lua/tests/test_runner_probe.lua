-- test_runner_probe.lua: regression probe for balanced-brace discovery.
-- An early inline return-table literal inside a helper must not truncate
-- discovery of the exported test table at the end of this file.

local assert = require("assert")

local function make_map_payload()
	return {
		topology = "square",
		cells = {
			{ x = 1, y = 2, z = 0, neighbors = {} },
		},
	}
end

local function test_probe_payload_shape()
	local payload = make_map_payload()
	assert.equals(payload.topology, "square")
	assert.equals(payload.cells[1].x, 1)
	assert.equals(payload.cells[1].y, 2)
end

local function test_probe_second_case()
	assert.is_true(true)
end

return {
	test_probe_payload_shape = test_probe_payload_shape,
	test_probe_second_case = test_probe_second_case,
}
