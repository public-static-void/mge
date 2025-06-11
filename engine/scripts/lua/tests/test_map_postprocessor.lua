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

-- === Map Validator Tests ===

local function test_validator_blocks_invalid_map()
    world:clear_map_validators()
    world:register_map_validator(function(map)
        -- Always fail
        return false
    end)
    local ok, err = pcall(function()
        world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
    end)
    assert.is_false(ok)
    assert.is_true(tostring(err):find("Map validator failed"))
end

local function test_validator_accepts_valid_map()
    world:clear_map_validators()
    local called = false
    world:register_map_validator(function(map)
        called = true
        return true
    end)
    world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
    assert.is_true(called)
end

local function test_clear_validators()
    local called = false
    world:register_map_validator(function(map)
        called = true
        return true
    end)
    world:clear_map_validators()
    world:apply_generated_map({ topology = "square", cells = { { x = 0, y = 0, z = 0 } } })
    assert.is_false(called)
end

return {
    test_postprocessor_invoked = test_postprocessor_invoked,
    test_postprocessor_error_blocks = test_postprocessor_error_blocks,
    test_clear_postprocessors = test_clear_postprocessors,
    test_validator_blocks_invalid_map = test_validator_blocks_invalid_map,
    test_validator_accepts_valid_map = test_validator_accepts_valid_map,
    test_clear_validators = test_clear_validators,
}
