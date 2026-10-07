-- Per-test isolated save paths for Lua save/load round-trip tests.
--
-- The test runner provisions a unique scratch directory per test and
-- exposes it as the MGE_TEST_TMP_DIR global (Lua runs without os/io,
-- so tests cannot build temp paths on their own). Save tests route
-- every file write through tmp_save_path instead of CWD-relative
-- fixed names, keeping the repo root free of byproducts and parallel
-- runs collision-free.
local M = {}

local counter = 0

function M.tmp_save_path(basename)
	assert(
		MGE_TEST_TMP_DIR ~= nil,
		"tmp_path: MGE_TEST_TMP_DIR is not set (run under mge_lua_test_runner)"
	)
	counter = counter + 1
	return MGE_TEST_TMP_DIR .. "/" .. basename .. "_" .. tostring(counter) .. ".json"
end

return M
