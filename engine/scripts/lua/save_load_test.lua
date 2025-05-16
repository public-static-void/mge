local e = spawn_entity()
set_component(e, "Health", { current = 99, max = 100 })
save_to_file("test_save.json")

despawn_entity(e)
print("Entities after remove:", #get_entities())
for _, id in ipairs(get_entities()) do
	print("Entity:", id)
end
assert(#get_entities() == 0)

load_from_file("test_save.json")
local entities = get_entities()
assert(#entities > 0)
local h = get_component(entities[1], "Health")
assert(h.current == 99)
os.remove("test_save.json")
