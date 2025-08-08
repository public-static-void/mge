local assert = require("assert")

-- Test assigning a move path and querying agent position and move_path
local function test_assign_move_path_and_queries()
	local e_agent = spawn_entity()

	-- Setup Position component with Square cell coordinates matching the map grid
	set_component(e_agent, "Position", { pos = { Square = { x = 0, y = 2, z = 0 } } })

	-- Use from_cell and to_cell that exist in the map (0,2,0) and (1,2,0)
	local from_cell = { Square = { x = 0, y = 2, z = 0 } }
	local to_cell = { Square = { x = 1, y = 2, z = 0 } }

	-- Call the Lua movement API function (to be implemented in the bindings)
	assign_move_path(e_agent, from_cell, to_cell)

	local agent_comp = get_component(e_agent, "Agent")

	assert.is_table(agent_comp, "Agent component should exist")
	assert.is_table(agent_comp.move_path, "move_path should be set")
	assert.is_true(#agent_comp.move_path > 0, "move_path should not be empty")

	-- is_agent_at_cell queries
	assert.is_true(is_agent_at_cell(e_agent, from_cell), "Agent should be at from_cell")
	assert.is_false(is_agent_at_cell(e_agent, to_cell), "Agent should NOT be at to_cell")

	-- is_move_path_empty query
	assert.is_false(is_move_path_empty(e_agent), "move_path should not be empty")
end

-- Test is_move_path_empty function for edge cases when move_path is missing or empty
local function test_is_move_path_empty_edge_cases()
	local e_agent = spawn_entity()

	-- No Agent component present, path should be considered empty
	assert.is_true(is_move_path_empty(e_agent), "No Agent component -> path is empty")

	-- Agent component exists but without move_path field (omit move_path to avoid schema errors)
	set_component(e_agent, "Agent", { entity_id = e_agent })
	assert.is_true(is_move_path_empty(e_agent), "Agent without move_path -> path is empty")

	-- Agent component with explicitly nil move_path field
	set_component(e_agent, "Agent", { entity_id = e_agent, move_path = nil })
	assert.is_true(is_move_path_empty(e_agent), "Agent with nil move_path -> path is empty")
end

return {
	test_assign_move_path_and_queries = test_assign_move_path_and_queries,
	test_is_move_path_empty_edge_cases = test_is_move_path_empty_edge_cases,
}
