local utils = require("utils")
local assert = {}

local function tostr(val)
	if type(val) == "table" then
		local s = "{"
		for k, v in pairs(val) do
			s = s .. tostring(k) .. "=" .. tostr(v) .. ", "
		end
		return s .. "}"
	else
		return tostring(val)
	end
end

function assert.equals(actual, expected, msg)
	if actual ~= expected then
		error((msg or "") .. ("\nassertion failed: expected %s, got %s"):format(tostr(expected), tostr(actual)), 2)
	end
end

function assert.not_nil(value, msg)
	if value == nil then
		error((msg or "") .. "\nassertion failed: value is nil", 2)
	end
end

function assert.is_nil(value, msg)
	if value ~= nil then
		error((msg or "") .. "\nassertion failed: value is not nil", 2)
	end
end

function assert.is_true(value, msg)
	if not value then
		error((msg or "") .. "\nassertion failed: value is not true", 2)
	end
end

function assert.is_false(value, msg)
	if value then
		error((msg or "") .. "\nassertion failed: value is not false", 2)
	end
end

function assert.is_table(value, msg)
	if type(value) ~= "table" then
		error((msg or "") .. "\nassertion failed: value is not a table (got " .. type(value) .. ")", 2)
	end
end

function assert.table_equals(tbl1, tbl2, msg)
	local function cmp(a, b)
		if type(a) ~= type(b) then
			return false
		end
		if type(a) ~= "table" then
			return a == b
		end
		for k, v in pairs(a) do
			if not cmp(v, b[k]) then
				return false
			end
		end
		for k, v in pairs(b) do
			if not cmp(v, a[k]) then
				return false
			end
		end
		return true
	end
	if not cmp(tbl1, tbl2) then
		error(
			(msg or "")
				.. ("\nassertion failed: tables not equal\nexpected: %s\ngot: %s"):format(tostr(tbl2), tostr(tbl1)),
			2
		)
	end
end

return assert
