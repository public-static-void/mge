-- Modular, generic Lua test runner with file/function filtering, colors, and robust summary

-- Argument parsing
local arg_mod_filter = arg and arg[1] or nil
local arg_func_filter = arg and arg[2] or nil

-- Color setup (ANSI escape codes)
local colors = {
	reset = "\27[0m",
	green = "\27[32m",
	red = "\27[31m",
	yellow = "\27[33m",
	cyan = "\27[36m",
}

local function color(c, s)
	return colors[c] .. s .. colors.reset
end

-- Directory detection
local test_dir = debug.getinfo(1, "S")
if test_dir then
	test_dir = test_dir.source:match("@(.*/)")
end
if not test_dir or test_dir == "" then
	test_dir = "engine/scripts/lua/tests/"
end
package.path = test_dir .. "?.lua;" .. package.path

-- Utility: error handler for stacktraces
local function traceback_handler(err)
	return debug.traceback(tostring(err), 2)
end

-- Utility: format test name
local function fmt_test(modname, fname)
	return ("%s.%s"):format(modname, fname)
end

-- Utility: extract main error line and location
local function extract_main_error(err)
	local msg = tostring(err):match("^[^\n]+") or tostring(err)
	local where = tostring(err):match("\n%s*([^\n]-%.lua:%d+):") or tostring(err):match("%.lua:%d+")
	if where then
		return msg, where
	else
		return msg
	end
end

-- Test discovery: find all test_*.lua files in test_dir
local test_modules = {}
local p = io.popen('ls "' .. test_dir .. '"')
if p then
	for file in p:lines() do
		if file:match("^test_.*%.lua$") then
			local modname = file:sub(1, -5)
			if not arg_mod_filter or modname:find(arg_mod_filter, 1, true) then
				table.insert(test_modules, modname)
			end
		end
	end
	p:close()
end

-- Test execution
local total, failed = 0, 0
local failed_tests = {}
local found_any = false

for _, modname in ipairs(test_modules) do
	local ok, mod = pcall(require, modname)
	if not ok then
		print(color("red", "[FAIL]   ") .. modname)
		print(color("red", "  " .. tostring(mod)))
		table.insert(failed_tests, { name = modname, err = tostring(mod) })
		failed = failed + 1
		found_any = true
	else
		for fname, fn in pairs(mod) do
			if type(fn) == "function" and fname:match("^test_") then
				if not arg_func_filter or fname:find(arg_func_filter, 1, true) then
					total = total + 1
					found_any = true
					local testname = fmt_test(modname, fname)
					io.write(color("cyan", "[RUN]    ") .. testname .. "\n")
					local ok2, err = xpcall(fn, traceback_handler)
					if ok2 then
						print(color("green", "[OK]     ") .. testname)
					else
						print(color("red", "[FAIL]   ") .. testname)
						print(color("yellow", err):gsub("\n", "\n  "))
						local msg, where = extract_main_error(err)
						table.insert(failed_tests, { name = testname, err = msg, where = where })
						failed = failed + 1
					end
				end
			end
		end
	end
end

-- Handle "no tests found" case
if not found_any then
	local filter = ""
	if arg_mod_filter and arg_func_filter then
		filter = ("%s %s"):format(arg_mod_filter, arg_func_filter)
	elseif arg_mod_filter then
		filter = arg_mod_filter
	elseif arg_func_filter then
		filter = arg_func_filter
	end
	print(color("red", ("No tests found matching filter: %s"):format(filter)))
	os.exit(2)
end

-- Summary and exit handling
print(color("cyan", string.rep("-", 50)))
print(("%d tests run, %d failed"):format(total, failed))

if failed > 0 then
	print(color("red", "\nFailed tests:"))
	for _, t in ipairs(failed_tests) do
		print(color("red", t.name))
		print("  " .. color("yellow", t.err))
		if t.where then
			print("  at " .. color("cyan", t.where))
		end
	end
	os.exit(1)
else
	print(color("green", "All tests passed!"))
	os.exit(0)
end
