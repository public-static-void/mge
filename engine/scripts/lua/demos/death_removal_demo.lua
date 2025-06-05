function dump(o, indent)
	indent = indent or ""
	if type(o) == "table" then
		local s = "{\n"
		for k, v in pairs(o) do
			s = s .. indent .. "  [" .. tostring(k) .. "] = " .. dump(v, indent .. "  ") .. ",\n"
		end
		return s .. indent .. "}"
	else
		return tostring(o)
	end
end

function print_corpses()
	local ids = get_entities_with_component("Corpse")
	if #ids == 0 then
		print("No Corpse components found.")
	else
		for _, id in ipairs(ids) do
			print("Entity", id, ":", dump(get_component(id, "Corpse")))
		end
	end
end

function print_all_healths()
	local ids = get_entities_with_component("Health")
	if #ids == 0 then
		print("No Health components found.")
	else
		for _, eid in ipairs(ids) do
			local h = get_component(eid, "Health")
			print(string.format("Entity %d: current = %.1f, max = %.1f", eid, h.current, h.max))
		end
	end
end

function print_all_positions()
	local ids = get_entities_with_component("Position")
	if #ids == 0 then
		print("No Position components found.")
	else
		for _, eid in ipairs(ids) do
			local pos = get_component(eid, "Position")
			print(string.format("Entity %d: %s", eid, dump(pos)))
		end
	end
end

function damage_all(amount)
	local ids = get_entities_with_component("Health")
	for _, eid in ipairs(ids) do
		local h = get_component(eid, "Health")
		h.current = math.max(0, h.current - amount)
		set_component(eid, "Health", h)
	end
end

local id = spawn_entity()
set_component(id, "Health", { current = 2.0, max = 10.0 })

print("Before damage:")
print_all_healths()

damage_all(3.0) -- kills the entity
process_deaths()

print("After death processing:")
print_all_healths()
print_all_positions()

for i = 1, 6 do
	process_decay()
	print("After decay tick " .. i)
	print_all_healths()
	print_all_positions()
	print_corpses()
end
