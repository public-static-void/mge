local luaunit = require("luaunit")

TestSaveLoad = {}

function TestSaveLoad:test_save_and_load()
	local e = spawn_entity()
	set_component(e, "Health", { current = 99, max = 100 })
	save_to_file("test_save.json")

	despawn_entity(e)
	luaunit.assertEquals(#get_entities(), 0, "Entities should be empty after despawn")

	load_from_file("test_save.json")
	local entities = get_entities()
	luaunit.assertTrue(#entities > 0, "No entities loaded from save file")
	local h = get_component(entities[1], "Health")
	luaunit.assertEquals(h.current, 99, "Loaded entity health.current mismatch")

	-- Clean up
	os.remove("test_save.json")
end

os.exit(luaunit.LuaUnit.run())
