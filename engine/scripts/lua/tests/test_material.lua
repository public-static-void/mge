-- test_material.lua: Tests for the material system.
-- Each test gets a fresh world via the test runner.

local assert = require("assert")

-- 1. get_material_properties returns correct values for wood
local function test_get_material_properties_wood()
    local props = get_material_properties("wood")
    assert.not_nil(props, "Material properties for wood should not be nil")
    assert.equals(props.density, 0.6, "Wood density should be 0.6")
    assert.equals(props.hardness, 2.0, "Wood hardness should be 2.0")
    assert.equals(props.flammability, 0.9, "Wood flammability should be 0.9")
end

-- 2. get_material_properties returns nil for unknown material
local function test_get_material_properties_unknown()
    local props = get_material_properties("nonexistent")
    assert.is_nil(props, "Unknown material should return nil")
end

-- 3. set_entity_material attaches component successfully
local function test_set_entity_material_success()
    local eid = spawn_entity()
    local err = set_entity_material(eid, "iron")
    assert.is_nil(err, "set_entity_material should not return error for known material")
    local mat = get_entity_material(eid)
    assert.not_nil(mat, "Material component should exist after set")
    assert.equals(mat.material, "iron", "Material should be 'iron'")
end

-- 4. set_entity_material returns error for unknown material
local function test_set_entity_material_unknown()
    local eid = spawn_entity()
    local err = set_entity_material(eid, "nonexistent")
    assert.not_nil(err, "set_entity_material should return error for unknown material")
end

-- 5. get_entity_material returns nil for entity without Material
local function test_get_entity_material_absent()
    local eid = spawn_entity()
    local mat = get_entity_material(eid)
    assert.is_nil(mat, "Entity without Material should return nil")
end

-- 6. get_material_names returns all material names
local function test_get_material_names()
    local names = get_material_names()
    assert.is_table(names, "Material names should be a table")
    assert.contains("wood", names, "Names should contain 'wood'")
    assert.contains("iron", names, "Names should contain 'iron'")
    assert.contains("steel", names, "Names should contain 'steel'")
    assert.contains("stone", names, "Names should contain 'stone'")
    assert.contains("leather", names, "Names should contain 'leather'")
    assert.contains("cloth", names, "Names should contain 'cloth'")
    assert.contains("bone", names, "Names should contain 'bone'")
    assert.contains("obsidian", names, "Names should contain 'obsidian'")
end

return {
    test_get_material_properties_wood = test_get_material_properties_wood,
    test_get_material_properties_unknown = test_get_material_properties_unknown,
    test_set_entity_material_success = test_set_entity_material_success,
    test_set_entity_material_unknown = test_set_entity_material_unknown,
    test_get_entity_material_absent = test_get_entity_material_absent,
    test_get_material_names = test_get_material_names,
}
