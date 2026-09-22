local assert = require("assert")

local function empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

local function test_get_temperature_returns_default()
	local temp = get_temperature()
	assert.equals(temp, 15.0)
end

local function test_set_temperature_round_trip()
	set_temperature(20.0)
	assert.equals(get_temperature(), 20.0)
end

local function test_set_temperature_clamps_high()
	set_temperature(100.0)
	assert.equals(get_temperature(), 60.0)
end

local function test_set_temperature_clamps_low()
	set_temperature(-100.0)
	assert.equals(get_temperature(), -60.0)
end

local function test_override_holds_across_tick()
	set_temperature(20.0)
	tick()
	assert.equals(get_temperature(), 20.0)
end

local function test_set_temperature_emits_changed_event()
	set_temperature(20.0)
	update_event_buses()
	local events = poll_ecs_event("temperature_changed")
	assert.equals(#events, 1)
	assert.equals(events[1].old_ambient, 15.0)
	assert.equals(events[1].new_ambient, 20.0)
end

local function test_natural_drift_emits_no_changed_event()
	tick()
	update_event_buses()
	local events = poll_ecs_event("temperature_changed")
	assert.equals(#events, 0)
end

local function test_body_exchange_drifts_toward_ambient()
	local e = spawn_entity()
	set_component(e, "Body", {
		parts = {
			{
				name = "torso",
				status = "healthy",
				kind = "flesh",
				temperature = 37.0,
				ideal_temperature = 37.0,
				insulation = 0.0,
				heat_loss = 0.0,
				children = empty_array(),
				equipped = empty_array(),
			},
		},
	})
	set_temperature(0.0)
	tick()
	local body = get_component(e, "Body")
	local torso = body.parts[1]
	assert.is_true(math.abs(torso.temperature - 35.15) < 0.0001, "expected ~35.15, got " .. tostring(torso.temperature))
	assert.is_true(math.abs(torso.heat_loss - 1.85) < 0.0001, "expected ~1.85, got " .. tostring(torso.heat_loss))
end

local function test_humidity_pressure_round_trip()
	assert.equals(get_humidity(), 0.5)
	assert.equals(get_pressure(), 1013.0)
	set_humidity(0.8)
	assert.equals(get_humidity(), 0.8)
	set_pressure(1000.0)
	assert.equals(get_pressure(), 1000.0)
end

local function test_humidity_pressure_clamps()
	set_humidity(2.0)
	assert.equals(get_humidity(), 1.0)
	set_humidity(-1.0)
	assert.equals(get_humidity(), 0.0)
	set_pressure(2000.0)
	assert.equals(get_pressure(), 1100.0)
	set_pressure(500.0)
	assert.equals(get_pressure(), 900.0)
end

local function test_humidity_pressure_shift_ambient()
	set_weather("Clear", 0.0, 100)
	set_humidity(0.5)
	set_pressure(1013.0)
	tick()
	local neutral = get_temperature()
	set_humidity(1.0)
	set_pressure(1013.0)
	tick()
	assert.is_true(
		math.abs((get_temperature() - neutral) - 3.0) < 0.5,
		"humid air should run ~3C hotter, got " .. tostring(get_temperature() - neutral)
	)
	set_humidity(0.5)
	set_pressure(973.0)
	tick()
	assert.is_true(
		math.abs((get_temperature() - neutral) + 2.0) < 0.5,
		"low pressure should run ~2C cooler, got " .. tostring(get_temperature() - neutral)
	)
end

return {
	test_get_temperature_returns_default = test_get_temperature_returns_default,
	test_set_temperature_round_trip = test_set_temperature_round_trip,
	test_set_temperature_clamps_high = test_set_temperature_clamps_high,
	test_set_temperature_clamps_low = test_set_temperature_clamps_low,
	test_override_holds_across_tick = test_override_holds_across_tick,
	test_set_temperature_emits_changed_event = test_set_temperature_emits_changed_event,
	test_natural_drift_emits_no_changed_event = test_natural_drift_emits_no_changed_event,
	test_body_exchange_drifts_toward_ambient = test_body_exchange_drifts_toward_ambient,
	test_humidity_pressure_round_trip = test_humidity_pressure_round_trip,
	test_humidity_pressure_clamps = test_humidity_pressure_clamps,
	test_humidity_pressure_shift_ambient = test_humidity_pressure_shift_ambient,
}
