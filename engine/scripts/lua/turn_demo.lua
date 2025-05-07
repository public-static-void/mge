local id = spawn_entity()
set_component(id, "Position", { x = 0.0, y = 0.0 })
set_component(id, "Health", { current = 10.0, max = 10.0 })

print_positions()
print_healths()
print("Turn: " .. get_turn())

tick()
print_positions()
print_healths()
print("Turn: " .. get_turn())
