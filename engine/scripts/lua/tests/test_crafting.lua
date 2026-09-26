local assert = require("assert")

local RECIPE = "iron_sword"

local function sword_recipe()
	return '{"name":"iron_sword","inputs":[{"kind":"iron","amount":2}],"outputs":[],"duration":3,'
		.. '"tools":[{"item":"hammer","consumed":false}],"materials":[{"material":"iron","amount":2}],'
		.. '"required_skill":{"skill":"crafting","level":2},'
		.. '"output_item":{"id":"iron_sword","name":"Iron Sword","slot":"hand"},"station":null,"xp":12}'
end

local function spawn_crafter(iron)
	local eid = spawn_entity()
	set_component(eid, "Stockpile", { resources = { iron = iron } })
	set_component(eid, "Inventory", { slots = { "hammer" }, max_slots = 10, weight = 0, volume = 0 })
	set_component(
		eid,
		"SkillLevels",
		{ skills = { crafting = 2 }, total_xp = 0, skill_xp = {}, skill_levels = {} }
	)
	return eid
end

local function setup()
	register_craft_recipe(RECIPE, sword_recipe())
	return spawn_crafter(200)
end

local function stockpile_iron(eid)
	return get_component(eid, "Stockpile").resources.iron
end

local function test_tool_gate_names_missing_hammer()
	local crafter = setup()
	set_component(crafter, "Inventory", { slots = {}, max_slots = 10, weight = 0, volume = 0 })

	local ok, err = can_craft(crafter, RECIPE)
	assert.is_false(ok, "gate without hammer should fail")
	assert.equals(err, "missing_tool:hammer", "gate must name the missing tool")

	local ok2, err2 = start_craft(crafter, RECIPE)
	assert.is_false(ok2, "start without hammer should fail")
	assert.equals(err2, "missing_tool:hammer", "start must name the missing tool")
	assert.is_nil(get_craft_state(crafter), "failed start must not create an order")

	set_component(crafter, "Inventory", { slots = { "hammer" }, max_slots = 10, weight = 0, volume = 0 })
	local ok3, err3 = can_craft(crafter, RECIPE)
	assert.is_true(ok3, "gate with hammer should pass")
	assert.is_nil(err3, "err should be nil on success")
end

local function test_consumed_tool_removed_once()
	register_craft_recipe(
		RECIPE,
		'{"name":"iron_sword","inputs":[],"outputs":[],"duration":3,'
			.. '"tools":[{"item":"hammer","consumed":true}],"materials":[],'
			.. '"output_item":{"id":"iron_sword","name":"Iron Sword","slot":"hand"}}'
	)
	local crafter = spawn_entity()
	set_component(crafter, "Inventory", { slots = { "hammer", "hammer" }, max_slots = 10, weight = 0, volume = 0 })

	local ok, err = start_craft(crafter, RECIPE)
	assert.is_true(ok, "start with two hammers should succeed, got " .. tostring(err))
	local slots = get_component(crafter, "Inventory").slots
	assert.equals(#slots, 1, "consumed tool must be removed exactly once")
	assert.equals(slots[1], "hammer", "one hammer must remain")
end

local function test_materials_deducted_and_output_spawned()
	local crafter = setup()
	local ok, err = start_craft(crafter, RECIPE)
	assert.is_true(ok, "start should succeed, got " .. tostring(err))
	assert.equals(stockpile_iron(crafter), 196, "inputs plus materials deduct at start")

	tick()
	tick()
	local mid = get_craft_state(crafter)
	assert.equals(mid.progress, 2, "two ticks must advance progress to 2")
	tick()

	local state = get_craft_state(crafter)
	assert.equals(state.state, "complete", "third tick must complete a duration-3 recipe")
	assert.equals(state.progress, 3, "completion stores terminal progress")
	local output = state.output_entity
	assert.not_nil(output, "completed order must record the output entity")

	local item = get_component(output, "Item")
	assert.equals(item.id, "iron_sword", "output item id mismatch")
	assert.equals(item.name, "Iron Sword", "output item name mismatch")
	assert.equals(item.slot, "hand", "output item slot mismatch")
	assert.equals(item.material, "iron", "output item material mismatch")
	local material = get_component(output, "Material")
	assert.equals(material.material, "iron", "output material key mismatch")
	assert.is_true(material.quality >= 0 and material.quality <= 10, "quality must be clamped to range")
end

local function test_skill_gate_and_xp()
	local crafter = setup()
	set_component(
		crafter,
		"SkillLevels",
		{ skills = { crafting = 1 }, total_xp = 0, skill_xp = {}, skill_levels = {} }
	)
	local ok, err = can_craft(crafter, RECIPE)
	assert.is_false(ok, "below-gate skill should fail")
	assert.equals(err, "insufficient_skill", "skill shortfall must report insufficient_skill")

	set_component(
		crafter,
		"SkillLevels",
		{ skills = { crafting = 2 }, total_xp = 0, skill_xp = {}, skill_levels = {} }
	)
	local ok2, err2 = start_craft(crafter, RECIPE)
	assert.is_true(ok2, "at-gate skill should start, got " .. tostring(err2))
	tick()
	tick()
	tick()

	-- No `update_event_buses()` here: `tick()` already swaps the buffers at
	-- tick end, and an extra swap would wipe the flushed read buffer while
	-- the write side is empty.
	local events = poll_ecs_event("craft_completed")
	assert.equals(#events, 1, "one craft_completed event expected")
	local payload = events[1]
	assert.equals(payload.entity, crafter, "event entity mismatch")
	assert.equals(payload.recipe, RECIPE, "event recipe mismatch")
	assert.not_nil(payload.output_entity, "event must carry output_entity")
	assert.not_nil(payload.output_item, "event must carry output_item")
	assert.equals(payload.material, "iron", "event material mismatch")
	assert.not_nil(payload.quality, "event must carry quality")
	assert.not_nil(payload.xp_gained, "event must carry xp_gained")
	assert.is_true(payload.xp_gained == 11 or payload.xp_gained == 12, "xp override 12 plus jitter floors to 11 or 12")

	local levels = get_component(crafter, "SkillLevels")
	assert.equals(levels.total_xp, payload.xp_gained, "xp grant must match the event")
	assert.equals(levels.skill_xp.crafting, payload.xp_gained, "per-skill xp must match the event")
end

local function test_error_parity_strings()
	local crafter = setup()

	local ok1, err1 = can_craft(crafter, "ghost_recipe")
	assert.is_false(ok1, "unknown recipe should fail")
	assert.equals(err1, "unknown_recipe", "unknown recipe string mismatch")

	local ok2, err2 = start_craft(crafter, RECIPE)
	assert.is_true(ok2, "first start should succeed, got " .. tostring(err2))
	local ok3, err3 = start_craft(crafter, RECIPE)
	assert.is_false(ok3, "double start should fail")
	assert.equals(err3, "already_crafting", "double start string mismatch")

	local other = spawn_entity()
	local ok4, err4 = cancel_craft(other)
	assert.is_false(ok4, "cancel without order should fail")
	assert.equals(err4, "no_craft_order", "missing order string mismatch")
end

local function test_cancel_refunds_and_emits()
	local crafter = setup()
	start_craft(crafter, RECIPE)
	assert.equals(stockpile_iron(crafter), 196, "start must deduct before cancel")

	local ok, err = cancel_craft(crafter)
	assert.is_true(ok, "cancel should succeed, got " .. tostring(err))
	assert.is_nil(err, "err should be nil on cancel")
	assert.equals(stockpile_iron(crafter), 200, "cancel must refund stockpile inputs")
	assert.is_nil(get_craft_state(crafter), "cancel must remove the order")

	update_event_buses()
	local events = poll_ecs_event("craft_cancelled")
	assert.equals(#events, 1, "one craft_cancelled event expected")
	assert.equals(events[1].entity, crafter, "cancel event entity mismatch")
	assert.equals(events[1].recipe, RECIPE, "cancel event recipe mismatch")
	assert.is_true(events[1].refunded, "cancel event must report refunded")
end

local function test_station_ignored_and_stockpile_only_unknown()
	register_craft_recipe(
		RECIPE,
		'{"name":"iron_sword","inputs":[],"outputs":[],"duration":1,'
			.. '"tools":[],"materials":[],"station":"bogus_workbench",'
			.. '"output_item":{"id":"iron_sword","name":"Iron Sword","slot":"hand"}}'
	)
	local crafter = spawn_entity()
	local ok, err = can_craft(crafter, RECIPE)
	assert.is_true(ok, "bogus station must not gate, got " .. tostring(err))

	register_craft_recipe(
		"plank_batch",
		'{"name":"plank_batch","inputs":[{"kind":"wood","amount":1}],'
			.. '"outputs":[{"kind":"plank","amount":2}],"duration":2}'
	)
	local ok2, err2 = can_craft(crafter, "plank_batch")
	assert.is_false(ok2, "stockpile-only recipe should fail on the craft path")
	assert.equals(err2, "unknown_recipe", "stockpile-only recipes stay unknown to craft")
end

local function test_full_inventory_leaves_world_entity()
	local crafter = setup()
	set_component(crafter, "Inventory", { slots = { "hammer" }, max_slots = 1, weight = 0, volume = 0 })
	start_craft(crafter, RECIPE)
	tick()
	tick()
	tick()

	-- Same no-extra-swap rule as above: the completion event was flushed by
	-- the tick itself.
	local events = poll_ecs_event("craft_completed")
	assert.equals(#events, 1, "completion must still fire when inventory is full")
	local output = events[1].output_entity
	assert.not_nil(output, "event must carry the world-entity fallback")
	assert.equals(get_component(output, "Item").id, "iron_sword", "fallback entity must carry the item")
	local slots = get_component(crafter, "Inventory").slots
	assert.equals(#slots, 1, "full inventory must not gain the output")
end

local function test_save_load_roundtrip_preserves_order()
	local crafter = setup()
	start_craft(crafter, RECIPE)
	tick()

	save_to_file("test_crafting_save.json")
	local entities = get_entities()
	for _, eid in ipairs(entities) do
		despawn_entity(eid)
	end
	load_from_file("test_crafting_save.json")

	local state = get_craft_state(crafter)
	assert.not_nil(state, "in-progress order must survive round-trip")
	assert.equals(state.recipe, RECIPE, "recipe must survive round-trip")
	assert.equals(state.progress, 1, "progress must survive round-trip")
	assert.equals(state.state, "in_progress", "order state must survive round-trip")
	assert.equals(stockpile_iron(crafter), 196, "deductions must survive round-trip")
	local recipes = list_craft_recipes()
	assert.equals(#recipes, 1, "one craft recipe must survive round-trip")
	assert.equals(recipes[1], RECIPE, "recipe name must survive round-trip")

	-- Post-load worlds carry no registered systems (`systems` is
	-- `#[serde(skip)]`, so `load_from_file` drops them — the vehicle
	-- round-trip test asserts preservation only for the same reason).
	-- Completion-after-load is proven by the Rust suite, which re-registers
	-- `CraftingSystem` explicitly; the bridge preserves everything needed
	-- to resume, as asserted above.
end

return {
	test_tool_gate_names_missing_hammer = test_tool_gate_names_missing_hammer,
	test_consumed_tool_removed_once = test_consumed_tool_removed_once,
	test_materials_deducted_and_output_spawned = test_materials_deducted_and_output_spawned,
	test_skill_gate_and_xp = test_skill_gate_and_xp,
	test_error_parity_strings = test_error_parity_strings,
	test_cancel_refunds_and_emits = test_cancel_refunds_and_emits,
	test_station_ignored_and_stockpile_only_unknown = test_station_ignored_and_stockpile_only_unknown,
	test_full_inventory_leaves_world_entity = test_full_inventory_leaves_world_entity,
	test_save_load_roundtrip_preserves_order = test_save_load_roundtrip_preserves_order,
}
