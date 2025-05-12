local e = spawn_entity()
set_component(e, "Health", { current = 99, max = 100 })
save_world("test_save.json")

remove_entity(e)
print("Entities after remove:", #get_entities())
for _, id in ipairs(get_entities()) do
	print("Entity:", id)
end
assert(#get_entities() == 0)

load_world("test_save.json")
local entities = get_entities()
assert(#entities > 0)
local h = get_component(entities[1], "Health")
assert(h.current == 99)
