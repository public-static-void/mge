-- test_item_registry.lua: Tests for item definition designer APIs.
-- Functions: register_item, get_item_definition, list_item_definitions,
--   load_item_definitions

local assert = require("assert")
local utils = require("utils")

-- register_item: valid item succeeds
local function test_register_item_valid()
	local ok, err = pcall(function()
		register_item('{"id":"iron_sword","name":"Iron Sword","slot":"weapon"}')
	end)
	assert.is_true(ok, "register_item should succeed for valid item: " .. tostring(err))
end

-- register_item: item missing 'id' field errors
local function test_register_item_missing_id()
	local ok, err = pcall(function()
		register_item('{"name":"No ID","slot":"weapon"}')
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("id"), "Error should mention missing id field")
end

-- register_item: item missing 'name' field errors
local function test_register_item_missing_name()
	local ok, err = pcall(function()
		register_item('{"id":"bad_item","slot":"weapon"}')
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("name"), "Error should mention missing name field")
end

-- register_item: item missing 'slot' field errors
local function test_register_item_missing_slot()
	local ok, err = pcall(function()
		register_item('{"id":"bad_item","name":"No Slot"}')
	end)
	assert.is_false(ok)
	local msg = utils.error_to_table(err)
	assert.is_true(msg.msg:find("slot"), "Error should mention missing slot field")
end

-- register_item: invalid JSON errors
local function test_register_item_invalid_json()
	local ok, err = pcall(function()
		register_item("not json at all")
	end)
	assert.is_false(ok)
end

-- register_item: duplicate ID overwrites previous
local function test_register_item_duplicate_overwrites()
	register_item('{"id":"dup_item","name":"First","slot":"weapon"}')
	register_item('{"id":"dup_item","name":"Second","slot":"shield"}')
	local def = get_item_definition("dup_item")
	assert.not_nil(def)
	assert.equals(def.name, "Second", "Duplicate registration should overwrite")
end

-- get_item_definition: returns table with correct fields
local function test_get_item_definition_returns_fields()
	register_item('{"id":"test_sword","name":"Test Sword","slot":"weapon"}')
	local def = get_item_definition("test_sword")
	assert.not_nil(def, "Registered item should be retrievable")
	assert.equals(def.id, "test_sword")
	assert.equals(def.name, "Test Sword")
	assert.equals(def.slot, "weapon")
end

-- get_item_definition: returns nil for nonexistent
local function test_get_item_definition_nonexistent()
	local def = get_item_definition("nonexistent_item_xyz")
	assert.is_nil(def, "Nonexistent item should return nil")
end

-- list_item_definitions: returns array of registered IDs
local function test_list_item_definitions()
	register_item('{"id":"list_a","name":"A","slot":"weapon"}')
	register_item('{"id":"list_b","name":"B","slot":"shield"}')
	local ids = list_item_definitions()
	assert.is_table(ids, "list_item_definitions should return a table")
	assert.contains("list_a", ids)
	assert.contains("list_b", ids)
end

-- load_item_definitions: errors on nonexistent directory
local function test_load_item_definitions_bad_dir()
	local ok, err = pcall(function()
		load_item_definitions("/nonexistent/path/to/items")
	end)
	assert.is_false(ok, "Loading from nonexistent directory should error")
end

return {
	test_register_item_valid = test_register_item_valid,
	test_register_item_missing_id = test_register_item_missing_id,
	test_register_item_missing_name = test_register_item_missing_name,
	test_register_item_missing_slot = test_register_item_missing_slot,
	test_register_item_invalid_json = test_register_item_invalid_json,
	test_register_item_duplicate_overwrites = test_register_item_duplicate_overwrites,
	test_get_item_definition_returns_fields = test_get_item_definition_returns_fields,
	test_get_item_definition_nonexistent = test_get_item_definition_nonexistent,
	test_list_item_definitions = test_list_item_definitions,
	test_load_item_definitions_bad_dir = test_load_item_definitions_bad_dir,
}
