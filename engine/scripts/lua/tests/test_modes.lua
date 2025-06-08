local assert = require("assert")

local function test_component_access_mode_enforcement()
	set_mode("colony")
	local id = spawn_entity()
	local ok1 = set_component(id, "Happiness", { base_value = 0.7 })
	local ok2 = set_component(id, "Inventory", { slots = {}, weight = 1.5, volume = 10 })
	assert.is_true(ok1, "Should be able to set Happiness in colony mode")
	assert.is_true(ok2, "Should be able to set Inventory in colony mode")
	-- If some components are expected to to fail, add:
	-- assert.is_false(set_component(id, "SomeRestrictedComponent", {...}))
end

return {
	test_component_access_mode_enforcement = test_component_access_mode_enforcement,
}
