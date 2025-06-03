local id = spawn_entity()
set_component(id, "Health", { current = 10.0, max = 10.0 })
print_healths()
for _, eid in ipairs(get_entities_with_component("Health")) do
	local h = get_component(eid, "Health")
	h.current = h.current - 3.0
	set_component(eid, "Health", h)
end
print_healths()
