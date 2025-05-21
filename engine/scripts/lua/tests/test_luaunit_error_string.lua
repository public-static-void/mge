local lu = require("luaunit")

function test_rust_error()
	local e = spawn_entity()
	set_inventory(e, { slots = {}, weight = 0.0, volume = 0.0 })
	local ok, err = pcall(function()
		remove_item_from_inventory(e, 1)
	end)
	print("ok:", ok, "err type:", type(err), "err:", tostring(err))
	-- Do not re-raise the error, just print for now
	lu.assertFalse(ok)
	lu.assertStrContains(tostring(err), "out of bounds")
end

os.exit(lu.LuaUnit.run())
