local assert = require("assert")

local function make_open_plane()
	local size = 10
	for x = -size, size do
		for y = -size, size do
			add_cell(x, y, 0)
		end
	end
	for x = -size, size do
		for y = -size, size do
			for dx = -1, 1 do
				for dy = -1, 1 do
					if dx ~= 0 or dy ~= 0 then
						local nx = x + dx
						local ny = y + dy
						if nx >= -size and nx <= size and ny >= -size and ny <= size then
							add_neighbor({ x = x, y = y, z = 0 }, { x = nx, y = ny, z = 0 })
						end
					end
				end
			end
		end
	end
end

local function set_species(eid, overrides)
	local base = {
		diet = "herbivore",
		graze_nutrition_rate = 0.2,
		metabolism_rate = 0.05,
		reproduction_threshold = 0.8,
		reproduction_cooldown_ticks = 10,
		litter_size = 1,
		detection_range = 6,
		noise_flee_threshold = 0.5,
		activity = "nocturnal",
	}
	if overrides ~= nil then
		for k, v in pairs(overrides) do
			base[k] = v
		end
	end
	set_component(eid, "Species", base)
end

local function spawn_wildlife(x, y, state, satiety, species_overrides)
	local eid = spawn_entity()
	set_component(eid, "Wildlife", {
		state = state,
		satiety = satiety,
		reproduction_cooldown = 0,
		flee_ticks = 0,
		rest_ticks = 0,
	})
	set_component(eid, "Position", { pos = { Square = { x = x, y = y, z = 0 } } })
	set_species(eid, species_overrides)
	return eid
end

local function wildlife_of(eid)
	return get_component(eid, "Wildlife")
end

local function test_species_wildlife_round_trip_with_defaults()
	local eid = spawn_entity()
	set_component(eid, "Species", {
		diet = "carnivore",
		graze_nutrition_rate = 0.2,
		metabolism_rate = 0.05,
		reproduction_threshold = 0.8,
		reproduction_cooldown_ticks = 10,
		litter_size = 1,
		detection_range = 6,
		noise_flee_threshold = 0.5,
		activity = "nocturnal",
	})
	local species = get_component(eid, "Species")
	assert.not_nil(species, "Species should round-trip")
	assert.equals(species.diet, "carnivore", "diet should round-trip")

	set_component(eid, "Wildlife", { state = "wander", satiety = 0.3 })
	local wildlife = wildlife_of(eid)
	assert.not_nil(wildlife, "Wildlife should round-trip")
	assert.equals(wildlife.state, "wander", "state should round-trip")
	assert.is_true(math.abs(wildlife.satiety - 0.3) < 0.000000001, "satiety should round-trip")
	assert.equals(wildlife.reproduction_cooldown, 0, "cooldown should default to 0")
	assert.equals(wildlife.flee_ticks, 0, "flee_ticks should default to 0")
	assert.equals(wildlife.rest_ticks, 0, "rest_ticks should default to 0")
end

local function test_nocturnal_graze_gain_exact_and_stationary()
	make_open_plane()
	local eid = spawn_wildlife(0, 0, "graze", 0.5, nil)

	tick()

	local wildlife = wildlife_of(eid)
	assert.is_true(math.abs(wildlife.satiety - 0.7) < 0.000000001, "spring gain must equal rate, got " .. tostring(wildlife.satiety))
	local pos = get_component(eid, "Position")
	assert.equals(pos.pos.Square.x, 0, "grazer must stay stationary")
	assert.equals(pos.pos.Square.y, 0, "grazer must stay stationary")
end

local function test_graze_gain_clamps_at_one()
	make_open_plane()
	local eid = spawn_wildlife(0, 0, "graze", 0.95, nil)
	set_component(eid, "Wildlife", {
		state = "graze",
		satiety = 0.95,
		reproduction_cooldown = 5,
		flee_ticks = 0,
		rest_ticks = 0,
	})

	tick()

	local wildlife = wildlife_of(eid)
	assert.equals(wildlife.satiety, 1.0, "satiety must clamp to 1.0, got " .. tostring(wildlife.satiety))
end

local function test_diurnal_rests_during_night()
	make_open_plane()
	local eid = spawn_wildlife(0, 0, "graze", 0.5, { activity = "diurnal" })

	tick()

	local wildlife = wildlife_of(eid)
	assert.equals(wildlife.state, "rest", "diurnal must rest at night")
	local pos = get_component(eid, "Position")
	assert.equals(pos.pos.Square.x, 0, "resting entity must stay stationary")
	assert.equals(pos.pos.Square.y, 0, "resting entity must stay stationary")
end

local function test_flee_on_noise()
	make_open_plane()
	local eid = spawn_wildlife(0, 0, "graze", 0.5, nil)
	emit_noise(eid, 1.0, 5)

	tick()

	local wildlife = wildlife_of(eid)
	assert.equals(wildlife.state, "flee", "noise above threshold must trigger flee")
	local pos = get_component(eid, "Position")
	local moved = pos.pos.Square.x ~= 0 or pos.pos.Square.y ~= 0
	assert.is_true(moved, "fleeing entity must leave its cell")
end

local function test_reproduce_spawns_child_and_updates_parent()
	make_open_plane()
	set_temperature(20.0)
	local parent = spawn_wildlife(0, 0, "graze", 0.9, nil)
	local before = #get_entities_with_component("Wildlife")

	tick()

	local after = #get_entities_with_component("Wildlife")
	assert.equals(after, before + 1, "one child must spawn")
	local wildlife = wildlife_of(parent)
	assert.is_true(math.abs(wildlife.satiety - 0.45) < 0.000000001, "parent satiety must halve, got " .. tostring(wildlife.satiety))
	assert.equals(wildlife.reproduction_cooldown, 10, "parent cooldown must reset")
	assert.equals(wildlife.state, "graze", "parent must return to graze")
	local child = nil
	for _, id in ipairs(get_entities_with_component("Wildlife")) do
		if id ~= parent then
			child = id
		end
	end
	assert.not_nil(child, "child entity must exist")
	local child_wildlife = wildlife_of(child)
	assert.equals(child_wildlife.state, "graze", "child must start grazing")
	assert.is_true(math.abs(child_wildlife.satiety - 0.5) < 0.000000001, "child satiety must start at 0.5")
	local child_species = get_component(child, "Species")
	assert.equals(child_species.diet, "herbivore", "child must copy parent diet")
end

return {
	test_species_wildlife_round_trip_with_defaults = test_species_wildlife_round_trip_with_defaults,
	test_nocturnal_graze_gain_exact_and_stationary = test_nocturnal_graze_gain_exact_and_stationary,
	test_graze_gain_clamps_at_one = test_graze_gain_clamps_at_one,
	test_diurnal_rests_during_night = test_diurnal_rests_during_night,
	test_flee_on_noise = test_flee_on_noise,
	test_reproduce_spawns_child_and_updates_parent = test_reproduce_spawns_child_and_updates_parent,
}
