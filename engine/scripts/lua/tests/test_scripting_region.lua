local assert = require("assert")

local function test_region()
	local eid1 = spawn_entity()
	set_component(eid1, "Region", { id = "room_1", kind = "room" })
	local eid2 = spawn_entity()
	set_component(eid2, "Region", { id = { "room_1", "biome_A" }, kind = "room" })
	local eid3 = spawn_entity()
	set_component(eid3, "Region", { id = "biome_A", kind = "biome" })

	local e_room = get_entities_in_region("room_1")
	assert.equals(#e_room, 2, "room_1 should have 2 entities")
	local e_biome = get_entities_in_region("biome_A")
	assert.equals(#e_biome, 2, "biome_A should have 2 entities")

	local e_kind_room = get_entities_in_region_kind("room")
	assert.equals(#e_kind_room, 2, "kind=room should have 2 entities")
	local e_kind_biome = get_entities_in_region_kind("biome")
	assert.equals(#e_kind_biome, 1, "kind=biome should have 1 entity")
end

return {
	test_region = test_region,
}
