-- engine/scripts/lua/world_postprocessors.lua

world_postprocessors = world_postprocessors or {}

function register_map_postprocessor(func)
	table.insert(world_postprocessors, func)
end

function run_world_postprocessors(world)
	for _, func in ipairs(world_postprocessors) do
		func(world)
	end
end
