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

local id = spawn_entity()
set_component(id, "Health", { current = 2.0, max = 10.0 })

print("Before damage:")
print_healths()

damage_all(3.0) -- kills the entity
process_deaths()

print("After death processing:")
print_healths()
print_positions()

for i = 1, 6 do
	process_decay()
	print("After decay tick " .. i)
	print_healths()
	print_positions()
	print_corpses()
end
