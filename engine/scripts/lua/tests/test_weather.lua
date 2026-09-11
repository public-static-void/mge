local assert = require("assert")

-- R016: get_weather() returns valid table with expected keys
local function test_get_weather_returns_valid_table()
	local weather = get_weather()
	assert.not_nil(weather)
	assert.equals(type(weather.condition), "string")
	assert.equals(type(weather.intensity), "number")
	assert.equals(type(weather.duration_remaining), "number")
end

-- R016: set_weather() changes weather and get_weather() reflects it
local function test_set_weather_changes_state()
	set_weather("rain", 0.8, 100)
	local weather = get_weather()
	assert.equals(weather.condition, "rain")
	assert.equals(weather.intensity, 0.8)
	assert.equals(weather.duration_remaining, 100)
end

-- R016: get_weather_visibility_modifier() returns correct value
local function test_visibility_modifier_matches_formula()
	set_weather("rain", 0.8, 100)
	tick()
	local vis = get_weather_visibility_modifier()
	-- 0.7 * 0.8 = 0.56; compare with tolerance for f64 rounding
	assert.is_true(math.abs(vis - 0.56) < 0.0001, "expected ~0.56, got " .. tostring(vis))
end

-- Set weather to clear: modifier = 1.0 (no reduction)
local function test_clear_has_full_visibility()
	set_weather("clear", 1.0, 100)
	tick()
	local vis = get_weather_visibility_modifier()
	assert.equals(vis, 1.0)
end

-- Unrecognized condition string maps to Clear per spec edge cases
local function test_invalid_condition_defaults_to_clear()
	set_weather("thunder", 0.5, 100)
	local weather = get_weather()
	assert.equals(weather.condition, "clear")
end

-- Clamping: intensity above 1.0 is clamped
local function test_intensity_clamped_to_1_0()
	set_weather("storm", 1.5, 100)
	local weather = get_weather()
	assert.equals(weather.intensity, 1.0)
end

-- Clamping: negative intensity is clamped to 0.0
local function test_intensity_clamped_to_0_0()
	set_weather("fog", -0.5, 100)
	local weather = get_weather()
	assert.equals(weather.intensity, 0.0)
end

-- Duration 0 forces transition on next tick
local function test_duration_zero_forces_transition()
	set_weather("cloudy", 0.5, 0)
	local before = get_weather()
	assert.equals(before.duration_remaining, 0)
	tick()
	local after = get_weather()
	-- After tick: duration expired, WeatherSystem transitions to new condition
	assert.is_true(after.duration_remaining > 0)
end

return {
	test_get_weather_returns_valid_table = test_get_weather_returns_valid_table,
	test_set_weather_changes_state = test_set_weather_changes_state,
	test_visibility_modifier_matches_formula = test_visibility_modifier_matches_formula,
	test_clear_has_full_visibility = test_clear_has_full_visibility,
	test_invalid_condition_defaults_to_clear = test_invalid_condition_defaults_to_clear,
	test_intensity_clamped_to_1_0 = test_intensity_clamped_to_1_0,
	test_intensity_clamped_to_0_0 = test_intensity_clamped_to_0_0,
	test_duration_zero_forces_transition = test_duration_zero_forces_transition,
}
