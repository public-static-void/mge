local assert = require("assert")

local function test_entities_in_cell()
	local eid = spawn_entity()
	set_component(eid, "Position", { pos = { Square = { x = 0, y = 0, z = 0 } } })
	local cell = { Square = { x = 0, y = 0, z = 0 } }
	local entities = entities_in_cell(cell)
	assert.equals(#entities, 1, "Should find one entity in cell")
end

local function test_count_entities_with_type()
	local id1 = spawn_entity()
	set_component(id1, "Type", { kind = "player" })
	local id2 = spawn_entity()
	set_component(id2, "Type", { kind = "enemy" })
	local id3 = spawn_entity()
	set_component(id3, "Type", { kind = "enemy" })

	local count_player = 0
	local count_enemy = 0
	for _, eid in ipairs(get_entities_with_component("Type")) do
		local t = get_component(eid, "Type")
		if t.kind == "player" then
			count_player = count_player + 1
		end
		if t.kind == "enemy" then
			count_enemy = count_enemy + 1
		end
	end
	assert.equals(count_player, 1)
	assert.equals(count_enemy, 2)
end

return {

	test_entities_in_cell = test_entities_in_cell,
	test_count_entities_with_type = test_count_entities_with_type,
}
