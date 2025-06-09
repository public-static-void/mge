local assert = require("assert")

local called = false

local function test_postprocessor_invoked()
	called = false
	world:clear_map_postprocessors()
	world:register_map_postprocessor(function(w)
		called = true
		assert.equals(w:get_map_cell_count(), 1)
	end)
	world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
	assert.is_true(called)
end

local function test_postprocessor_error_blocks()
	world:clear_map_postprocessors()
	world:register_map_postprocessor(function(w)
		error("fail on purpose")
	end)
	local ok, err = pcall(function()
		world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
	end)
	assert.is_false(ok)
	assert.is_true(tostring(err):find("fail on purpose"))
end

local function test_clear_postprocessors()
	local called2 = false
	world:register_map_postprocessor(function(w)
		called2 = true
	end)
	world:clear_map_postprocessors()
	world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
	assert.is_false(called2)
end

return {
	test_postprocessor_invoked = test_postprocessor_invoked,
	test_postprocessor_error_blocks = test_postprocessor_error_blocks,
	test_clear_postprocessors = test_clear_postprocessors,
}
