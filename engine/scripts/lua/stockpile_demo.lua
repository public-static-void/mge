-- Pretty-print any Lua table (recursive, indented)
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

local entity = spawn_entity()

-- Initialize stockpile with some resources
set_component(entity, "Stockpile", { resources = { wood = 10, stone = 5 } })

print("Initial stockpile:")
print(dump(get_component(entity, "Stockpile")))

-- Add 3 food
modify_stockpile_resource(entity, "food", 3)
print("After adding food:")
print(dump(get_component(entity, "Stockpile")))

-- Remove 2 wood
modify_stockpile_resource(entity, "wood", -2)
print("After removing wood:")
print(dump(get_component(entity, "Stockpile")))

-- Try removing too much stone (should error)
local ok, err = pcall(function()
	modify_stockpile_resource(entity, "stone", -10)
end)
if not ok then
	print("Error removing stone (expected!):", err)
end
