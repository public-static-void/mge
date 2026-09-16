local assert = require("assert")

local function test_zone_eight_op_sequence()
	set_mode("colony")

	local zid = designate_zone("stockpile", "depot", { rect = { x0 = 0, y0 = 0, z = 0, x1 = 1, y1 = 1 } })
	assert.not_nil(zid, "designate_zone should return a zone id")

	assert.equals(true, assign_cells_to_zone(zid, { { Square = { x = 5, y = 5, z = 0 } } }))
	assert.equals(
		true,
		assign_cells_to_zone(zid, { { Square = { x = 5, y = 5, z = 0 } } }),
		"duplicate assign should be idempotent"
	)

	local zones = list_zones()
	assert.equals(1, #zones, "one zone should be listed")
	assert.equals(zid, zones[1].id)
	assert.equals("depot", zones[1].label)
	assert.equals("stockpile", zones[1].kind)
	assert.equals(5, zones[1].cell_count, "rect 2x2 plus one assigned cell")

	local zone = get_zone(zid)
	assert.not_nil(zone, "get_zone should return the zone")
	assert.equals(zid, zone.id)
	assert.equals("stockpile", zone.kind)

	assert.equals(true, rename_zone(zid, "store"))
	assert.equals(true, set_zone_kind(zid, "farm"))
	local renamed = get_zone(zid)
	assert.equals("store", renamed.label)
	assert.equals("farm", renamed.kind)

	local kind_cells = get_cells_in_region_kind("farm")
	assert.equals(5, #kind_cells, "rekind moves kind-query membership")
	assert.equals(0, #get_cells_in_region_kind("stockpile"), "old kind should be empty")

	assert.equals(true, unassign_cells_from_zone(zid, { { Square = { x = 5, y = 5, z = 0 } } }))
	assert.equals(
		true,
		unassign_cells_from_zone(zid, { { Square = { x = 9, y = 9, z = 0 } } }),
		"unassign of an unmembered cell should be idempotent"
	)
	assert.equals(4, #get_cells_in_region(zid), "only rect cells should remain")

	assert.equals(true, remove_zone(zid), "remove should succeed")
	assert.is_nil(get_zone(zid), "removed zone should be gone")
	assert.equals(false, remove_zone(zid), "second remove should return false")
	assert.equals(false, rename_zone(zid, "ghost"), "rename of unknown id should return false")
end

local function test_zone_cell_list_designate()
	set_mode("colony")

	local zid = designate_zone("farm", nil, {
		cells = {
			{ Square = { x = 0, y = 0, z = 0 } },
			{ Square = { x = 1, y = 0, z = 0 } },
		},
	})
	assert.not_nil(zid, "cell-list designate should return a zone id")
	assert.equals(2, #get_cells_in_region(zid), "both listed cells should resolve")
	assert.equals(2, #get_cells_in_region_kind("farm"), "kind query should expose the cells")
end

local function test_zone_mode_gate_outside_colony()
	set_mode("roguelike")

	local ok_designate =
		pcall(designate_zone, "stockpile", nil, { rect = { x0 = 0, y0 = 0, z = 0, x1 = 1, y1 = 1 } })
	assert.is_false(ok_designate, "designate_zone outside colony mode should error")

	local ok_remove = pcall(remove_zone, "zone-1")
	assert.is_false(ok_remove, "remove_zone outside colony mode should error")

	local ok_rename = pcall(rename_zone, "zone-1", "store")
	assert.is_false(ok_rename, "rename_zone outside colony mode should error")

	local ok_rekind = pcall(set_zone_kind, "zone-1", "farm")
	assert.is_false(ok_rekind, "set_zone_kind outside colony mode should error")

	local ok_assign = pcall(assign_cells_to_zone, "zone-1", {})
	assert.is_false(ok_assign, "assign_cells_to_zone outside colony mode should error")

	local ok_unassign = pcall(unassign_cells_from_zone, "zone-1", {})
	assert.is_false(ok_unassign, "unassign_cells_from_zone outside colony mode should error")

	assert.not_nil(list_zones(), "list_zones should stay ungated")
	assert.is_nil(get_zone("zone-1"), "get_zone should stay ungated and return nil")
end

return {
	test_zone_eight_op_sequence = test_zone_eight_op_sequence,
	test_zone_cell_list_designate = test_zone_cell_list_designate,
	test_zone_mode_gate_outside_colony = test_zone_mode_gate_outside_colony,
}
