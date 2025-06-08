local assert = require("assert")

local function test_get_user_input()
	-- Patch get_user_input for this test only
	_G.get_user_input = function(prompt)
		return "test_value"
	end

	local input = get_user_input("Prompt: ")
	assert.not_nil(input, "get_user_input returned nil")
	assert.equals(type(input), "string", "get_user_input did not return a string")
	assert.equals(input, "test_value", "get_user_input did not return the expected test value")
end

return {
	test_get_user_input = test_get_user_input,
}
