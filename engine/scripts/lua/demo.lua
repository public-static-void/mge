local id = spawn_entity()
set_component(id, "Position", { x = 1.1, y = 2.2 })
local pos = get_component(id, "Position")
print("From file: pos.x=" .. tostring(pos.x) .. " pos.y=" .. tostring(pos.y))
