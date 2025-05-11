local e1 = spawn_entity()
set_component(e1, "Health", { current = 10, max = 10 })
set_component(e1, "Position", { x = 1, y = 2 })

local e2 = spawn_entity()
set_component(e2, "Health", { current = 5, max = 10 })

local e3 = spawn_entity()
set_component(e3, "Position", { x = 3, y = 4 })

local both = get_entities_with_components({ "Health", "Position" })
assert(#both == 1 and both[1] == e1, "Multi-component query failed")
