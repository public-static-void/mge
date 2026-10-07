local assert = require("assert")

local function params(seed)
	local p = {}
	p.seed = seed
	p.width = 20
	p.height = 15
	return p
end

local function deep_equal(a, b)
	if type(a) ~= type(b) then
		return false
	end
	if type(a) ~= "table" then
		return a == b
	end
	for k, v in pairs(a) do
		if not deep_equal(v, b[k]) then
			return false
		end
	end
	for k, v in pairs(b) do
		if not deep_equal(v, a[k]) then
			return false
		end
	end
	return true
end

local function test_builtin_algorithms_listed()
	local names = list_worldgen_plugins()
	assert.contains("dungeon", names)
	assert.contains("caves", names)
end

local function test_switch_algorithms_by_name()
	local dungeon_map = invoke_worldgen_plugin("dungeon", params(42))
	local caves_map = invoke_worldgen_plugin("caves", params(42))
	assert.equals(dungeon_map.topology, "square")
	assert.equals(caves_map.topology, "square")
	assert.is_true(#dungeon_map.cells > 0, "dungeon map has cells")
	assert.is_true(#caves_map.cells > 0, "caves map has cells")
end

local function test_caves_deterministic_for_fixed_seed()
	local first = invoke_worldgen_plugin("caves", params(7))
	local second = invoke_worldgen_plugin("caves", params(7))
	assert.table_equals(first, second)
end

local function test_caves_differs_across_seeds()
	local seed_a = invoke_worldgen_plugin("caves", params(42))
	local seed_b = invoke_worldgen_plugin("caves", params(99))
	assert.is_false(deep_equal(seed_a, seed_b), "different seeds must produce different caves")
end

return {
	test_builtin_algorithms_listed = test_builtin_algorithms_listed,
	test_switch_algorithms_by_name = test_switch_algorithms_by_name,
	test_caves_deterministic_for_fixed_seed = test_caves_deterministic_for_fixed_seed,
	test_caves_differs_across_seeds = test_caves_differs_across_seeds,
}
