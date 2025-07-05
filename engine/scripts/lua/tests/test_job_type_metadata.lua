local assert = require("assert")

local function test_get_job_type_metadata()
	local job_types = get_job_types()
	assert.is_table(job_types)
	assert.contains("DigTunnel", job_types)

	local meta = get_job_type_metadata("DigTunnel")
	assert.is_table(meta)
	assert.equals(meta.name, "DigTunnel")
	assert.is_table(meta.effects)
	assert.is_table(meta.requirements)
	assert.equals(meta.requirements[1], "Tool:Pickaxe")
	assert.equals(meta.duration, 5)
	assert.equals(meta.effects[1].action, "ModifyTerrain")
	assert.equals(meta.effects[1].from, "rock")
	assert.equals(meta.effects[1].to, "tunnel")
end

return {
	test_get_job_type_metadata = test_get_job_type_metadata,
}
