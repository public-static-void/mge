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
end
