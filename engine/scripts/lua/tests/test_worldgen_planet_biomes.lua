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

local function assert_biome_shape(map)
	assert.equals(map.topology, "square")
	assert.is_true(#map.cells > 0, "planet_biomes map has cells")
	for i, cell in ipairs(map.cells) do
		assert.is_true(
			type(cell.biome) == "string" and cell.biome ~= "",
			"cell " .. i .. " carries a non-empty biome label"
		)
	end
end

local function test_planet_biomes_listed()
	local names = list_worldgen_plugins()
	assert.contains("planet_biomes", names)
end

local function test_switch_to_planet_biomes_by_name()
	local planet_map = invoke_worldgen_plugin("planet_biomes", params(42))
	assert_biome_shape(planet_map)
end

local function test_planet_biomes_deterministic_for_fixed_seed()
	local first = invoke_worldgen_plugin("planet_biomes", params(7))
	local second = invoke_worldgen_plugin("planet_biomes", params(7))
	assert.table_equals(first, second)
end

local function test_planet_biomes_differs_across_seeds()
	local seed_a = invoke_worldgen_plugin("planet_biomes", params(42))
	local seed_b = invoke_worldgen_plugin("planet_biomes", params(99))
	assert.is_false(deep_equal(seed_a, seed_b), "different seeds must produce different planet biomes")
end

local function test_planet_biomes_default_size_succeeds()
	local defaulted = invoke_worldgen_plugin("planet_biomes", { seed = 42 })
	assert_biome_shape(defaulted)
	assert.equals(#defaulted.cells, 32 * 32)
end

return {
	test_planet_biomes_listed = test_planet_biomes_listed,
	test_switch_to_planet_biomes_by_name = test_switch_to_planet_biomes_by_name,
	test_planet_biomes_deterministic_for_fixed_seed = test_planet_biomes_deterministic_for_fixed_seed,
	test_planet_biomes_differs_across_seeds = test_planet_biomes_differs_across_seeds,
	test_planet_biomes_default_size_succeeds = test_planet_biomes_default_size_succeeds,
}
