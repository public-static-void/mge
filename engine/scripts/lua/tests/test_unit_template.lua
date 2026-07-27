-- test_unit_template.lua: Tests for unit template designer APIs.
-- Functions: register_unit_template, spawn_from_template, get_unit_template,
--   list_unit_templates

local assert = require("assert")
local utils = require("utils")

-- register_unit_template: valid template succeeds
local function test_register_unit_template_valid()
	local json = '{"name":"test_warrior","version":"1.0.0","components":{"Type":{"kind":"player"},"Health":{"current":100,"max":100}}}'
	local ok, err = pcall(function()
		register_unit_template("test_warrior", json)
	end)
	assert.is_true(ok, "register_unit_template should succeed: " .. tostring(err))
end

-- register_unit_template: invalid JSON errors
local function test_register_unit_template_invalid_json()
	local ok, err = pcall(function()
		register_unit_template("bad", "not json at all")
	end)
	assert.is_false(ok, "Invalid JSON should error")
end

-- get_unit_template: returns table with correct fields
local function test_get_unit_template_returns_fields()
	local json = '{"name":"tmpl_a","version":"1.0.0","components":{"Type":{"kind":"npc"}}}'
	register_unit_template("tmpl_a", json)
	local tmpl = get_unit_template("tmpl_a")
	assert.not_nil(tmpl, "Registered template should be retrievable")
	assert.equals(tmpl.name, "tmpl_a")
	assert.equals(tmpl.version, "1.0.0")
end

-- get_unit_template: returns nil for nonexistent
local function test_get_unit_template_nonexistent()
	local tmpl = get_unit_template("nonexistent_template_xyz")
	assert.is_nil(tmpl, "Nonexistent template should return nil")
end

-- list_unit_templates: includes registered templates
local function test_list_unit_templates()
	register_unit_template("list_tmpl_1", '{"name":"list_tmpl_1","version":"1.0.0","components":{}}')
	register_unit_template("list_tmpl_2", '{"name":"list_tmpl_2","version":"1.0.0","components":{}}')
	local names = list_unit_templates()
	assert.is_table(names, "list_unit_templates should return a table")
	assert.contains("list_tmpl_1", names)
	assert.contains("list_tmpl_2", names)
end

-- spawn_from_template: creates entity with template components
local function test_spawn_from_template_basic()
	local json = '{"name":"spawn_test","version":"1.0.0","components":{"Type":{"kind":"creature"},"Health":{"current":50,"max":50}}}'
	register_unit_template("spawn_test", json)
	local eid = spawn_from_template("spawn_test")
	assert.not_nil(eid, "spawn_from_template should return entity ID")
	local type = get_component(eid, "Type")
	assert.not_nil(type, "Entity should have Type component")
	assert.equals(type.kind, "creature")
	local health = get_component(eid, "Health")
	assert.not_nil(health, "Entity should have Health component")
	assert.equals(health.current, 50)
end

-- spawn_from_template: nonexistent template errors
local function test_spawn_from_template_nonexistent()
	local ok, err = pcall(function()
		spawn_from_template("no_such_template")
	end)
	assert.is_false(ok, "Spawning nonexistent template should error")
end

-- spawn_from_template: overrides deep-merge into component data
local function test_spawn_from_template_overrides()
	local json = '{"name":"override_test","version":"1.0.0","components":{"Type":{"kind":"animal"},"Health":{"current":30,"max":30}}}'
	register_unit_template("override_test", json)
	local eid = spawn_from_template("override_test", { Health = { current = 10 } })
	local health = get_component(eid, "Health")
	assert.equals(health.current, 10, "Override should replace current")
	assert.equals(health.max, 30, "Non-overridden field should remain")
end

-- spawn_from_template: template with multiple components
local function test_spawn_from_template_multi_component()
	local json = '{"name":"multi_tmpl","version":"1.0.0","components":{"Type":{"kind":"npc"},"Health":{"current":75,"max":75},"Sight":{"range":15}}}'
	register_unit_template("multi_tmpl", json)
	local eid = spawn_from_template("multi_tmpl")
	assert.not_nil(get_component(eid, "Type"))
	assert.not_nil(get_component(eid, "Health"))
	assert.not_nil(get_component(eid, "Sight"))
	local sight = get_component(eid, "Sight")
	assert.equals(sight.range, 15)
end

-- load_unit_templates: errors on nonexistent directory
local function test_load_unit_templates_bad_dir()
	local ok, err = pcall(function()
		load_unit_templates("/nonexistent/path/to/templates")
	end)
	assert.is_false(ok, "Loading from nonexistent directory should error")
end

-- load_unit_templates: loads from engine/assets/templates
local function test_load_unit_templates_engine_dir()
	local ok, err = pcall(function()
		load_unit_templates("engine/assets/templates")
	end)
	assert.is_true(ok, "Loading engine templates should succeed: " .. tostring(err))
	local names = list_unit_templates()
	assert.is_true(#names > 0, "Engine templates should register at least one template")
end

return {
	test_register_unit_template_valid = test_register_unit_template_valid,
	test_register_unit_template_invalid_json = test_register_unit_template_invalid_json,
	test_get_unit_template_returns_fields = test_get_unit_template_returns_fields,
	test_get_unit_template_nonexistent = test_get_unit_template_nonexistent,
	test_list_unit_templates = test_list_unit_templates,
	test_spawn_from_template_basic = test_spawn_from_template_basic,
	test_spawn_from_template_nonexistent = test_spawn_from_template_nonexistent,
	test_spawn_from_template_overrides = test_spawn_from_template_overrides,
	test_spawn_from_template_multi_component = test_spawn_from_template_multi_component,
	test_load_unit_templates_bad_dir = test_load_unit_templates_bad_dir,
	test_load_unit_templates_engine_dir = test_load_unit_templates_engine_dir,
}
