-- test_trade.lua: Tests for ungated transfers and treaty-gated trades.
-- Each test gets a fresh world via the test runner.
-- Global functions: transfer_stockpile_resource, has_active_trade_treaty,
-- execute_treaty_trade (plus spawn_entity, set_component,
-- get_stockpile_resources, set_faction, propose_treaty, accept_treaty,
-- declare_war, declare_peace)

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

local function assert_transfer_error_contains(ok, err, variant, msg)
	assert.is_false(ok, msg)
	assert.is_true(
		string.find(tostring(err) or "", variant, 1, true) ~= nil,
		(msg or "") .. " (error should mention " .. variant .. ")"
	)
end

function test_transfer_moves_balance()
	local a, b = make_stocked("grain", 10.0, 0.0)
	transfer_stockpile_resource(a, b, "grain", 4.0)
	assert.equals(balances(a, "grain"), 6.0, "Source should hold 6.0 after moving 4.0")
	assert.equals(balances(b, "grain"), 4.0, "Destination should hold 4.0 after receiving 4.0")
end

function test_transfer_insufficient_leaves_unchanged()
	local a, b = make_stocked("grain", 10.0, 0.0)
	local ok, err = pcall(transfer_stockpile_resource, a, b, "grain", 99.0)
	assert_transfer_error_contains(ok, err, "InsufficientFunds", "Overdraft should fail")
	assert.equals(balances(a, "grain"), 10.0, "Source should be unchanged after failed transfer")
	assert.equals(balances(b, "grain"), 0.0, "Destination should be unchanged after failed transfer")
end

function test_transfer_rejects_bad_kind_and_amount()
	local a, b = make_stocked("grain", 10.0, 0.0)
	local nan = 0 / 0

	local ok, err = pcall(transfer_stockpile_resource, a, b, "", 1.0)
	assert_transfer_error_contains(ok, err, "UnknownKind", "Empty kind should fail")

	for _, amount in ipairs({ 0.0, -1.0, nan }) do
		local ok2, err2 = pcall(transfer_stockpile_resource, a, b, "grain", amount)
		assert_transfer_error_contains(ok2, err2, "NonPositiveAmount", "Amount " .. tostring(amount) .. " should fail")
	end

	local inf = math.huge
	local ok3, err3 = pcall(transfer_stockpile_resource, a, b, "grain", inf)
	assert_transfer_error_contains(ok3, err3, "NonPositiveAmount", "Infinite amount should fail")

	assert.equals(balances(a, "grain"), 10.0, "Source should be unchanged after rejected transfers")
	assert.equals(balances(b, "grain"), 0.0, "Destination should be unchanged after rejected transfers")
end

function test_transfer_missing_stockpile_errors()
	local a, b = make_stocked("grain", 10.0, 0.0)
	local bare = spawn_entity()

	local ok, err = pcall(transfer_stockpile_resource, bare, b, "grain", 1.0)
	assert_transfer_error_contains(ok, err, "NoStockpile", "Missing source stockpile should fail")

	local ok2, err2 = pcall(transfer_stockpile_resource, a, bare, "grain", 1.0)
	assert_transfer_error_contains(ok2, err2, "NoStockpile", "Missing destination stockpile should fail")

	assert.equals(balances(a, "grain"), 10.0, "Source should be unchanged after NoStockpile failure")
	assert.equals(balances(b, "grain"), 0.0, "Destination should be unchanged after NoStockpile failure")
end

function test_transfer_self_is_validated_noop()
	local a, _ = make_stocked("grain", 10.0, 0.0)
	transfer_stockpile_resource(a, a, "grain", 2.0)
	assert.equals(balances(a, "grain"), 10.0, "Self-transfer should leave the balance unchanged")
end

local function make_treaty_pair()
	local a, b = make_stocked("grain", 10.0, 0.0)
	set_faction(a, "f1", "member")
	set_faction(b, "f2", "member")
	return a, b
end

function test_treaty_lifecycle_gates_execution()
	local a, b = make_treaty_pair()
	assert.is_false(has_active_trade_treaty("f1", "f2"), "No treaty should be active before proposing")

	local tid = propose_treaty("f1", "f2", "trade", nil)
	accept_treaty(tid)
	assert.is_true(has_active_trade_treaty("f1", "f2"), "Accepted trade treaty should read active")
	assert.is_true(has_active_trade_treaty("f2", "f1"), "Treaty lookup should ignore party order")

	execute_treaty_trade(a, b, "grain", 3.0)
	assert.equals(balances(a, "grain"), 7.0, "Gated trade should move grain while treaty is live")
	assert.equals(balances(b, "grain"), 3.0, "Gated trade should credit the destination")

	declare_war("f1", "f2")
	assert.is_true(has_active_trade_treaty("f1", "f2"), "Treaty record should survive declaration of war")
	local ok, err = pcall(execute_treaty_trade, a, b, "grain", 1.0)
	assert_transfer_error_contains(ok, err, "RelationIsWar", "Gated trade during war should fail")
	assert.equals(balances(a, "grain"), 7.0, "Source should be unchanged after war-blocked trade")
	assert.equals(balances(b, "grain"), 3.0, "Destination should be unchanged after war-blocked trade")

	declare_peace("f1", "f2")
	execute_treaty_trade(a, b, "grain", 1.0)
	assert.equals(balances(a, "grain"), 6.0, "Gated trade should succeed again after peace")
	assert.equals(balances(b, "grain"), 4.0, "Destination should credit again after peace")
end

function test_gated_trade_without_treaty_fails()
	local a, b = make_treaty_pair()
	local ok, err = pcall(execute_treaty_trade, a, b, "grain", 1.0)
	assert_transfer_error_contains(ok, err, "NoLiveTreaty", "Gated trade without a treaty should fail")
	assert.equals(balances(a, "grain"), 10.0, "Source should be unchanged without a treaty")
	assert.equals(balances(b, "grain"), 0.0, "Destination should be unchanged without a treaty")
end

function test_gated_trade_without_faction_fails()
	local a, b = make_stocked("grain", 10.0, 0.0)
	local ok, err = pcall(execute_treaty_trade, a, b, "grain", 1.0)
	assert_transfer_error_contains(ok, err, "NoLiveTreaty", "Gated trade without factions should fail")
	assert.equals(balances(a, "grain"), 10.0, "Source should be unchanged without factions")
	assert.equals(balances(b, "grain"), 0.0, "Destination should be unchanged without factions")
end

-- Upkeep/consumption parity: generic component path plus per-tick drain.

local function make_upkeep_entity(stock, drains)
	local e = spawn_entity()
	set_component(e, "Stockpile", { resources = stock })
	set_component(e, "Upkeep", { drains = drains })
	return e
end

function test_upkeep_roundtrip_and_rejection()
	local e = spawn_entity()
	set_component(e, "Upkeep", { drains = { { kind = "grain", amount_per_tick = 2.0 } } })
	local up = get_component(e, "Upkeep")
	assert.equals(up.drains[1].kind, "grain", "Upkeep round-trip should keep the drain kind")
	assert.equals(up.drains[1].amount_per_tick, 2.0, "Upkeep round-trip should keep the drain amount")

	local bad_cases = {
		{ drains = { { kind = "", amount_per_tick = 2.0 } } },
		{ drains = { { kind = "grain", amount_per_tick = 0.0 } } },
		{ drains = { { kind = "grain", amount_per_tick = -1.0 } } },
		{ drains = { { kind = "grain", amount_per_tick = 0 / 0 } } },
	}
	for _, bad in ipairs(bad_cases) do
		local ok, _ = pcall(set_component, e, "Upkeep", bad)
		assert.is_false(ok, "Invalid upkeep drain should be rejected")
	end
end

function test_upkeep_drains_each_tick()
	local e = make_upkeep_entity({ grain = 10.0 }, { { kind = "grain", amount_per_tick = 2.0 } })
	tick()
	assert.equals(balances(e, "grain"), 8.0, "One tick should drain 2.0 grain")
	tick()
	tick()
	assert.equals(balances(e, "grain"), 4.0, "Three ticks should drain 6.0 grain total")
end

function test_upkeep_shortage_keeps_balance_and_emits_event()
	local e = make_upkeep_entity({ grain = 1.0 }, { { kind = "grain", amount_per_tick = 2.0 } })
	tick()
	assert.equals(balances(e, "grain"), 1.0, "Shorted entity should keep its balance")
	local events = poll_ecs_event("consumption_shortage")
	assert.equals(#events, 1, "Shortage should emit exactly one event")
	assert.equals(events[1].entity, e, "Shortage event should name the drained entity")
	assert.equals(events[1].kind, "grain", "Shortage event should name the missing kind")
	assert.equals(events[1].required, 2.0, "Shortage event should carry the required amount")
	assert.equals(events[1].available, 1.0, "Shortage event should carry the available amount")
end

function test_upkeep_multi_drain_is_atomic()
	local e = make_upkeep_entity(
		{ grain = 10.0, wood = 1.0 },
		{ { kind = "grain", amount_per_tick = 2.0 }, { kind = "wood", amount_per_tick = 2.0 } }
	)
	tick()
	assert.equals(balances(e, "grain"), 10.0, "Atomic shortage should leave grain untouched")
	assert.equals(balances(e, "wood"), 1.0, "Atomic shortage should leave wood untouched")
	local events = poll_ecs_event("consumption_shortage")
	assert.equals(#events, 1, "Atomic shortage should emit exactly one event")
	assert.equals(events[1].kind, "wood", "Shortage event should name the missing kind")
end

function test_upkeep_absent_entity_is_spared()
	local e = spawn_entity()
	set_component(e, "Stockpile", { resources = { grain = 10.0 } })
	tick()
	assert.equals(balances(e, "grain"), 10.0, "Entity without Upkeep should never be drained")
end

return {
	test_transfer_moves_balance = test_transfer_moves_balance,
	test_transfer_insufficient_leaves_unchanged = test_transfer_insufficient_leaves_unchanged,
	test_transfer_rejects_bad_kind_and_amount = test_transfer_rejects_bad_kind_and_amount,
	test_transfer_missing_stockpile_errors = test_transfer_missing_stockpile_errors,
	test_transfer_self_is_validated_noop = test_transfer_self_is_validated_noop,
	test_treaty_lifecycle_gates_execution = test_treaty_lifecycle_gates_execution,
	test_gated_trade_without_treaty_fails = test_gated_trade_without_treaty_fails,
	test_gated_trade_without_faction_fails = test_gated_trade_without_faction_fails,
	test_upkeep_roundtrip_and_rejection = test_upkeep_roundtrip_and_rejection,
	test_upkeep_drains_each_tick = test_upkeep_drains_each_tick,
	test_upkeep_shortage_keeps_balance_and_emits_event = test_upkeep_shortage_keeps_balance_and_emits_event,
	test_upkeep_multi_drain_is_atomic = test_upkeep_multi_drain_is_atomic,
	test_upkeep_absent_entity_is_spared = test_upkeep_absent_entity_is_spared,
}
