local assert = require("assert")
local utils = require("utils")

local function test_job_children_and_dependencies()
	init_job_event_logger()
	local parent_id = spawn_entity()
	assign_job(parent_id, "ParentJob", { state = "pending", category = "test" })

	-- Set children
	local child1 = { job_type = "ChildJob", state = "pending", category = "test", progress = 0.0 }
	local child2 = { job_type = "ChildJob", state = "pending", category = "test", progress = 0.0 }
	set_job_children(parent_id, { child1, child2 })

	-- Get children back and check
	local children = get_job_children(parent_id)
	assert.is_table(children)
	assert.equals(2, #children)
	assert.equals("ChildJob", children[1].job_type)
	assert.equals("ChildJob", children[2].job_type)

	-- Set dependencies (complex expr)
	local deps = {
		all_of = {
			"job:fetch_wood",
			{ any_of = { "job:mine_stone", "job:collect_clay" } },
			{ ["not"] = { "job:destroyed_bridge" } },
		},
	}
	set_job_dependencies(parent_id, deps)

	-- Get dependencies back and check
	local got_deps = get_job_dependencies(parent_id)
	assert.is_table(got_deps)
	assert.is_table(got_deps.all_of)
	assert.equals("job:fetch_wood", got_deps.all_of[1])
	assert.is_table(got_deps.all_of[2].any_of)
	assert.equals("job:mine_stone", got_deps.all_of[2].any_of[1])
	assert.equals("job:collect_clay", got_deps.all_of[2].any_of[2])
	assert.equals("job:destroyed_bridge", got_deps.all_of[3]["not"][1])

	-- Overwrite children with empty array
	set_job_children(parent_id, utils.empty_array())
	children = get_job_children(parent_id)
	assert.is_table(children)
	assert.equals(0, #children)

	-- Overwrite dependencies with a simple string array
	set_job_dependencies(parent_id, { "job:foo", "job:bar" })
	got_deps = get_job_dependencies(parent_id)
	assert.is_table(got_deps)
	assert.equals("job:foo", got_deps[1])
	assert.equals("job:bar", got_deps[2])
end

return {
	test_job_children_and_dependencies = test_job_children_and_dependencies,
}
