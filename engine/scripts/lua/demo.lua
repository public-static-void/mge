local id = spawn_entity()
set_position(id, 1.1, 2.2)
local pos = get_position(id)
print("From file: pos.x=" .. tostring(pos.x) .. " pos.y=" .. tostring(pos.y))
