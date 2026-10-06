-- test_supply.lua: Tests for the five-function supply bridge surface.
-- Each test gets a fresh world via the test runner.
-- Global functions: create_supply_link, remove_supply_link,
-- list_supply_links, set_supply_link_active, get_supply_link
-- (plus spawn_entity, set_component, get_stockpile_resources,
-- tick, poll_ecs_event)

local assert = require("assert")

local function make_stocked(kind, a_amount, b_amount)
	local a = spawn_entity()
	local b = spawn_entity()
	set_component(a, "Stockpile", { resources = { [kind] = a_amount } })
	set_component(b, "Stockpile", { resources = { [kind] = b_amount } })
	return a, b
end

local function balances(eid, kind)
	local res = get_stockpile_resources(eid)
	if res == nil then
		return nil
	end
	return res[kind]
end

local function assert_supply_error_contains(ok, err, variant, msg)
	assert.is_false(ok, msg)
	assert.is_true(
		string.find(tostring(err) or "", variant, 1, true) ~= nil,
		(msg or "") .. " (error should mention " .. variant .. ")"
	)
end

function test_create_link_returns_record_verbatim()
	local a, b = make_stocked("grain", 100.0, 0.0)
	local link = create_supply_link(a, b, "grain", 10.0, 25.0)
	local rec = get_supply_link(link)
	assert.equals(rec.source, a, "Record should keep the source endpoint")
	assert.equals(rec.target, b, "Record should keep the target endpoint")
	assert.equals(rec.kind, "grain", "Record should keep the resource kind")
	assert.equals(rec.amount_per_tick, 10.0, "Record should keep the per-tick amount")
	assert.equals(rec.capacity_per_tick, 25.0, "Record should keep the per-tick capacity")
	assert.is_true(rec.active, "New links should start active")
end

function test_create_link_rejects_bad_input()
	local a, b = make_stocked("grain", 100.0, 0.0)
	local bare = spawn_entity()
	local nan = 0 / 0
	local inf = math.huge

	local ok, err = pcall(create_supply_link, a, a, "grain", 10.0, 25.0)
	assert_supply_error_contains(ok, err, "SameEndpoint", "Self-links should fail")

	local ok2, err2 = pcall(create_supply_link, a, b, "", 10.0, 25.0)
	assert_supply_error_contains(ok2, err2, "UnknownKind", "Empty kind should fail")

	for _, amount in ipairs({ 0.0, -1.0, nan, inf }) do
		local oka, erra = pcall(create_supply_link, a, b, "grain", amount, 25.0)
		assert_supply_error_contains(oka, erra, "NonPositiveAmount", "Amount " .. tostring(amount) .. " should fail")
		local okc, errc = pcall(create_supply_link, a, b, "grain", 10.0, amount)
		assert_supply_error_contains(okc, errc, "NonPositiveAmount", "Capacity " .. tostring(amount) .. " should fail")
	end

	local oks, errs = pcall(create_supply_link, bare, b, "grain", 10.0, 25.0)
	assert_supply_error_contains(oks, errs, "NoStockpile", "Bare source should fail")

	local okt, errt = pcall(create_supply_link, a, bare, "grain", 10.0, 25.0)
	assert_supply_error_contains(okt, errt, "NoStockpile", "Bare target should fail")

	assert.equals(#list_supply_links(), 0, "Rejected creations should leave no links behind")
end

function test_unknown_link_errors()
	local bogus = 999999
	local ok, err = pcall(remove_supply_link, bogus)
	assert_supply_error_contains(ok, err, "UnknownLink", "Removing an unknown link should fail")

	local ok2, err2 = pcall(set_supply_link_active, bogus, false)
	assert_supply_error_contains(ok2, err2, "UnknownLink", "Toggling an unknown link should fail")

	local ok3, err3 = pcall(get_supply_link, bogus)
	assert_supply_error_contains(ok3, err3, "UnknownLink", "Reading an unknown link should fail")
end

function test_list_returns_links_in_ascending_order()
	local a, b = make_stocked("grain", 100.0, 0.0)
	local c = spawn_entity()
	set_component(c, "Stockpile", { resources = { grain = 50.0 } })
	local first = create_supply_link(a, b, "grain", 10.0, 25.0)
	local second = create_supply_link(b, c, "grain", 5.0, 5.0)
	local links = list_supply_links()
	assert.equals(#links, 2, "Two links should be listed")
	assert.is_true(links[1] < links[2], "Links should list in ascending id order")
	assert.equals(links[1], math.min(first, second), "First entry should be the lower id")
	assert.equals(links[2], math.max(first, second), "Second entry should be the higher id")
end

function test_remove_link_drops_it_from_list_and_query()
	local a, b = make_stocked("grain", 100.0, 0.0)
	local link = create_supply_link(a, b, "grain", 10.0, 25.0)
	assert.equals(#list_supply_links(), 1, "One link should be listed before removal")
	remove_supply_link(link)
	assert.equals(#list_supply_links(), 0, "No links should be listed after removal")
	local ok, err = pcall(get_supply_link, link)
	assert_supply_error_contains(ok, err, "UnknownLink", "Removed link should no longer be readable")
end

function test_tick_delivers_capped_request()
	local a, b = make_stocked("grain", 100.0, 0.0)
	create_supply_link(a, b, "grain", 10.0, 25.0)
	tick()
	assert.equals(balances(a, "grain"), 90.0, "One tick should move 10 grain off the source")
	assert.equals(balances(b, "grain"), 10.0, "One tick should credit 10 grain to the target")
	local events = poll_ecs_event("supply_delivered")
	assert.equals(#events, 1, "Delivery should emit exactly one event")
	assert.equals(events[1].kind, "grain", "Delivery event should name the kind")
	assert.equals(events[1].amount, 10.0, "Delivery event should carry the moved amount")
end

function test_tick_respects_capacity_cap()
	local a, b = make_stocked("grain", 100.0, 0.0)
	create_supply_link(a, b, "grain", 30.0, 12.0)
	tick()
	assert.equals(balances(a, "grain"), 88.0, "The capacity cap should bind instead of the amount")
	assert.equals(balances(b, "grain"), 12.0, "The target should receive exactly the capped request")
end

function test_inactive_link_skips_silently_then_resumes()
	local a, b = make_stocked("grain", 100.0, 0.0)
	local link = create_supply_link(a, b, "grain", 10.0, 25.0)
	set_supply_link_active(link, false)
	assert.is_false(get_supply_link(link).active, "Toggle should flip the stored flag off")
	tick()
	assert.equals(balances(a, "grain"), 100.0, "Inactive link should move nothing off the source")
	assert.equals(balances(b, "grain"), 0.0, "Inactive link should credit nothing to the target")
	assert.equals(#poll_ecs_event("supply_delivered"), 0, "Inactive link should emit no delivery")
	assert.equals(#poll_ecs_event("supply_shortfall"), 0, "Inactive link should emit no shortfall")
	assert.equals(#poll_ecs_event("supply_blocked"), 0, "Inactive link should emit no block")
	set_supply_link_active(link, true)
	assert.is_true(get_supply_link(link).active, "Toggle should flip the stored flag on")
	tick()
	assert.equals(balances(a, "grain"), 90.0, "Reactivated link should deliver on the next tick")
	assert.equals(balances(b, "grain"), 10.0, "Reactivated link should credit the target")
end

return {
	test_create_link_returns_record_verbatim = test_create_link_returns_record_verbatim,
	test_create_link_rejects_bad_input = test_create_link_rejects_bad_input,
	test_unknown_link_errors = test_unknown_link_errors,
	test_list_returns_links_in_ascending_order = test_list_returns_links_in_ascending_order,
	test_remove_link_drops_it_from_list_and_query = test_remove_link_drops_it_from_list_and_query,
	test_tick_delivers_capped_request = test_tick_delivers_capped_request,
	test_tick_respects_capacity_cap = test_tick_respects_capacity_cap,
	test_inactive_link_skips_silently_then_resumes = test_inactive_link_skips_silently_then_resumes,
}
