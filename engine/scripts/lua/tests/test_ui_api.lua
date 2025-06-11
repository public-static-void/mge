local assert = require("assert")

local function test_create_button_widget()
	local id = ui.create_widget("Button", { label = "OK", pos = { 1, 2 }, color = { 255, 255, 255 } })
	assert.not_nil(id, "ui.create_widget should return a widget id")
	assert.is_true(id > 0, "widget id should be positive")
end

local function test_load_json_button()
	local ids = ui.load_json([[{
        "type": "Button",
        "props": { "label": "Root", "pos": [0, 0], "color": [255, 255, 255] }
    }]])
	assert.not_nil(ids, "ui.load_json should return a table of ids")
	assert.is_table(ids, "ui.load_json should return a table")
	assert.is_true(#ids >= 1, "ui.load_json should return at least one id")
end

local function test_remove_widget()
	local id = ui.create_widget("Button", { label = "RemoveMe", pos = { 0, 0 }, color = { 10, 10, 10 } })
	assert.is_true(id > 0, "Widget id should be positive")

	-- Remove the widget, should succeed
	local removed = ui.remove_widget(id)
	assert.is_true(removed, "Widget should be removed")

	-- Removing again should return false
	local removed_again = ui.remove_widget(id)
	assert.is_false(removed_again, "Widget should not be removed twice")

	-- Removing a random id should return false
	assert.is_false(ui.remove_widget(999999999), "Non-existent widget should not be removed")
end

local function test_set_widget_props()
	local id = ui.create_widget("Button", { label = "Old", pos = { 0, 0 }, color = { 1, 2, 3 } })
	assert.is_true(id > 0, "Widget id should be positive")

	-- Update properties
	local updated = ui.set_widget_props(id, { label = "New", pos = { 5, 6 }, color = { 7, 8, 9 } })
	assert.is_true(updated, "Widget should be updated")

	-- Update non-existent widget
	assert.is_false(ui.set_widget_props(999999999, { label = "X" }), "Non-existent widget should not be updated")
end

local function test_get_widget_props()
	local id = ui.create_widget("Button", { label = "QueryMe", pos = { 3, 4 }, color = { 10, 20, 30 } })
	local props = ui.get_widget_props(id)
	assert.not_nil(props, "get_widget_props should return a table")
	assert.equals(props.label, "QueryMe", "Label should match")
	assert.is_table(props.pos, "Pos should be a table")
	assert.equals(props.pos[1], 3)
	assert.equals(props.pos[2], 4)
end

local function test_add_remove_child_panel()
	local panel_id = ui.create_widget("Panel", { pos = { 0, 0 } })
	local btn_id = ui.create_widget("Button", { label = "Child", pos = { 1, 1 }, color = { 1, 2, 3 } })
	assert.is_true(ui.add_child(panel_id, btn_id), "Should add child to panel")
	-- Now remove it
	assert.is_true(ui.remove_child(panel_id, btn_id), "Should remove child from panel")
	-- Remove again should fail
	assert.is_false(ui.remove_child(panel_id, btn_id), "Should not remove child twice")
end

local function test_set_callback_and_trigger()
	local id = ui.create_widget("Button", { label = "CB", pos = { 1, 2 }, color = { 1, 2, 3 } })
	local called = false
	ui.set_callback(id, "click", function(widget_id)
		called = widget_id == id
	end)
	ui.trigger_event(id, "click", { x = 1, y = 2 })
	assert.is_true(called, "Callback should be called on click event")
end

local function test_focus_widget()
	local id = ui.create_widget("Button", { label = "Focus", pos = { 1, 2 }, color = { 1, 2, 3 } })
	assert.is_true(ui.focus_widget(id), "Should be able to focus widget")
end

return {
	test_create_button_widget = test_create_button_widget,
	test_load_json_button = test_load_json_button,
	test_remove_widget = test_remove_widget,
	test_set_widget_props = test_set_widget_props,
	test_get_widget_props = test_get_widget_props,
	test_add_remove_child_panel = test_add_remove_child_panel,
	test_set_callback_and_trigger = test_set_callback_and_trigger,
	test_focus_widget = test_focus_widget,
}
