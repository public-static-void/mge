-- MGE Narrative Demo — Food Shortage incident
--
-- Playable end-to-end via mge_cli (no input required):
--   make run-cli ARGS="engine/scripts/lua/demos/narrative_food_shortage.lua"
--
-- The CLI preloads the shipped scenario content, so this script only drives
-- the incident loop: tick until the food-shortage decision fires, present it,
-- resolve one consequential choice, and show the observable aftermath.

print("=== MGE Narrative Demo: Food Shortage ===")

local scenarios = list_scenarios()
print("Registered scenarios: " .. tostring(#scenarios))
for _, s in ipairs(scenarios) do
	print("- " .. s.id .. ": " .. s.name)
end

local pending = {}
for _ = 1, 12 do
	tick()
	pending = poll_pending_decisions()
	if #pending > 0 then
		break
	end
end

if #pending == 0 then
	print("ERROR: no incident fired within 12 ticks (turn " .. tostring(get_turn()) .. ")")
	return
end

local decision = pending[1]
local fired = get_scenario(decision.scenario_id)
print("Turn " .. tostring(get_turn()) .. ": INCIDENT — " .. fired.name)
print(fired.description)
print("The council weighs its choices:")
for _, c in ipairs(decision.choices) do
	print("  [" .. c.id .. "] " .. c.label)
end

local before = get_standing("colonists", "leadership")
print("Colonist standing before: " .. tostring(before))

resolve_decision(decision.id, "send_foragers")
print("Resolved: foragers sent into the wilds.")

print("Colonist standing after: " .. tostring(get_standing("colonists", "leadership")))
print("Pending decisions: " .. tostring(#poll_pending_decisions()))

local history = get_narrative_history()
print("History records: " .. tostring(#history))
for _, r in ipairs(history) do
	local line = "- turn " .. tostring(r.turn) .. " " .. r.scenario_id .. " " .. r.kind
	if r.choice_id then
		line = line .. " (" .. r.choice_id .. ")"
	end
	print(line)
end

print("Demo complete.")
