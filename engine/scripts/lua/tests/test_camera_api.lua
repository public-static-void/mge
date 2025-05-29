local assert = require("assert")

local function test_camera_api()
	-- Set camera position
	set_camera(3, 7)
	local cam = get_camera()
	assert.is_table(cam, "get_camera() should return a table")
	assert.equals(cam.x, 3, "Camera x should be 3 after set_camera")
	assert.equals(cam.y, 7, "Camera y should be 7 after set_camera")

	-- Move camera again
	set_camera(10, 2)
	local cam2 = get_camera()
	assert.equals(cam2.x, 10, "Camera x should be 10 after set_camera")
	assert.equals(cam2.y, 2, "Camera y should be 2 after set_camera")
end

return { test_camera_api = test_camera_api }
