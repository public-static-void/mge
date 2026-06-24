local assert = require("assert")

local function test_time_of_day_advances()
	local initial = get_time_of_day()
	assert.equals(initial.hour, 0)
	assert.equals(initial.minute, 0)
	assert.equals(initial.day, 0)
	assert.equals(initial.season, "spring")

	tick()
	local after_tick = get_time_of_day()
	assert.equals(after_tick.minute, 1)

	for _ = 1, 59 do
		tick()
	end
	local after_hour = get_time_of_day()
	assert.equals(after_hour.hour, 1)
	assert.equals(after_hour.minute, 0)
end

local function test_day_increments_after_full_day()
	for _ = 1, 24 * 60 do
		tick()
	end
	local tod = get_time_of_day()
	assert.equals(tod.day, 1)
	assert.equals(tod.hour, 0)
	assert.equals(tod.minute, 0)
	assert.equals(tod.season, "spring")
end

local function test_season_changes_to_summer()
	for _ = 1, 30 * 24 * 60 do
		tick()
	end
	local tod = get_time_of_day()
	assert.equals(tod.day, 30)
	assert.equals(tod.season, "summer")
end

return {
	test_time_of_day_advances = test_time_of_day_advances,
	test_day_increments_after_full_day = test_day_increments_after_full_day,
	test_season_changes_to_summer = test_season_changes_to_summer,
}
