local function print_all_healths()
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

local id = spawn_entity()
set_component(id, "Health", { current = 10.0, max = 10.0 })
print_all_healths()
for _, eid in ipairs(get_entities_with_component("Health")) do
	local h = get_component(eid, "Health")
	h.current = h.current - 3.0
	set_component(eid, "Health", h)
end
print_all_healths()
