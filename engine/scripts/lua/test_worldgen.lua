-- Register a Lua worldgen function
register_worldgen("luagen", function(params)
	assert(type(params) == "table")
	assert(params.width == 7)
	return { cells = { { id = "luacell", x = 1, y = 2 } } }
end)

-- List should include our plugin
local names = list_worldgen()
assert(type(names) == "table")
local found = false
for _, name in ipairs(names) do
	if name == "luagen" then
		found = true
	end
end
assert(found, "luagen should be in worldgen plugin list")

-- Invocation should call our function and return the expected structure
local result = invoke_worldgen("luagen", { width = 7 })
assert(result.cells[1].id == "luacell")
