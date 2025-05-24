local assert = require("assert")
local utils = require("utils")

local function test_save_and_load()
	-- Setup: create some entities and save them
	local e1 = spawn_entity()
	set_inventory(e1, { slots = utils.empty_array(), weight = 0.0, volume = 0.0 })
	local e2 = spawn_entity()
	set_component(e2, "Health", { current = 10, max = 10 })

	save_to_file("test_save.json")
	-- Despawn all entities in the world
	local entities = get_entities()
	for _, eid in ipairs(entities) do
		despawn_entity(eid)
	end

	local entities_after_despawn = get_entities()
	assert.equals(#entities_after_despawn, 0, "Entities should be empty after despawn")

	-- Restore
	load_from_file("test_save.json")
	local entities_after = get_entities()
	assert.is_true(#entities_after >= 2, "Entities should exist after loading")
end

return {
	test_save_and_load = test_save_and_load,
}
