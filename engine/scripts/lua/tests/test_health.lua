local assert = require("assert")

local function test_health_component()
	local id = spawn_entity()
	set_component(id, "Health", { current = 7.0, max = 10.0 })
	local health = get_component(id, "Health")
	assert.equals(health.current, 7.0)
	assert.equals(health.max, 10.0)
end

local function test_batch_health_reduction()
	local id1 = spawn_entity()
	set_component(id1, "Health", { current = 10.0, max = 10.0 })
	local id2 = spawn_entity()
	set_component(id2, "Health", { current = 5.0, max = 8.0 })

	for _, eid in ipairs(get_entities_with_component("Health")) do
		local h = get_component(eid, "Health")
		h.current = h.current - 3.0
		set_component(eid, "Health", h)
	end

	local h1 = get_component(id1, "Health")
	local h2 = get_component(id2, "Health")
	assert.equals(h1.current, 7.0)
	assert.equals(h2.current, 2.0)
end

return {
	test_health_component = test_health_component,
	test_batch_health_reduction = test_batch_health_reduction,
}
