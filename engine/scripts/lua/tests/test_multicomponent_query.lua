local assert = require("assert")

local function test_query()
	local e1 = spawn_entity()
	set_component(e1, "Health", { current = 10, max = 10 })
	set_component(e1, "Position", { pos = { Square = { x = 1, y = 2, z = 0 } } })

	local e2 = spawn_entity()
	set_component(e2, "Health", { current = 5, max = 10 })

	local e3 = spawn_entity()
	set_component(e3, "Position", { pos = { Square = { x = 3, y = 4, z = 0 } } })

	local both = get_entities_with_components({ "Health", "Position" })
	assert.equals(#both, 1, "Multi-component query failed: wrong number of entities")
	assert.equals(both[1], e1, "Multi-component query failed: wrong entity returned")
end

return {
	test_query = test_query,
}
