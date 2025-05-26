local assert = require("assert")

local function test_time_of_day_advances()
	local initial = get_time_of_day()
	assert.equals(initial.hour, 0)
	assert.equals(initial.minute, 0)

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

return { test_time_of_day_advances = test_time_of_day_advances }
