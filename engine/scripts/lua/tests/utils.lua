-- utils.lua: Helper functions for tests

local utils = {}

-- Returns a table that will be serialized as a JSON array (not object)
function utils.empty_array()
	local t = {}
	return setmetatable(t, { __is_array = true })
end

-- Converts various Lua error values to a table with a 'msg' field
function utils.error_to_table(err)
	if type(err) == "string" then
		local ok, decoded = pcall(function()
			return require("json").decode(err)
		end)
		if ok and type(decoded) == "table" then
			return decoded
		end
		return { msg = err }
	elseif type(err) == "table" then
		return err
	else
		return { msg = tostring(err) }
	end
end

return utils
