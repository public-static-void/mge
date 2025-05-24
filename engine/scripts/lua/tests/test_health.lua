local assert = require("assert")

local function test_health_component()
	local id = spawn_entity()
	set_component(id, "Health", { current = 7.0, max = 10.0 })
	local health = get_component(id, "Health")
	assert.equals(health.current, 7.0)
	assert.equals(health.max, 10.0)
end

return {
	test_health_component = test_health_component,
}
