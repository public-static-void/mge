local assert = require("assert")

local function test_advance_job_progress()
	init_job_event_logger()
	local eid = spawn_entity()
	local agent_id = spawn_entity()
	-- Add Agent component with the appropriate skill for "TestJob"
	set_component(agent_id, "Agent", { entity_id = agent_id, skills = { TestJob = 1.0 } })

	assign_job(eid, "TestJob", { state = "pending", progress = 0.0, category = "test", assigned_to = agent_id })

	local jobs = list_jobs()
	assert.equals(1, #jobs)
	local job_id = jobs[1].id

	local job = get_job(job_id)
	assert.equals("pending", job.state)
	assert.equals(0.0, job.progress)

	-- Advance the job once
	advance_job_state(job_id)

	job = get_job(job_id)
	assert.is_true(job.state == "in_progress" or job.state == "pending")
	assert.is_true(job.progress > 0.0)

	-- Advance multiple times to complete the job
	for _ = 1, 10 do
		advance_job_state(job_id)
	end

	job = get_job(job_id)
	assert.equals("complete", job.state)
	assert.is_true(job.progress >= 3.0)

	-- Advancing a completed job should not change state
	local prev_progress = job.progress
	advance_job_state(job_id)
	job = get_job(job_id)
	assert.equals("complete", job.state)
	assert.equals(prev_progress, job.progress)
end

-- Skill-multiplier regression test: higher skill level yields proportionally higher progress.
-- Verifies that skill=5.0 produces progress > 4x skill=1.0 (matching R020's pattern).
local function test_skill_multiplier_regression()
	init_job_event_logger()

	-- Agent A: high skill (5.0) in TestJob via SkillLevels
	local agent_a = spawn_entity()
	set_component(agent_a, "SkillLevels", {
		skills = { TestJob = 5.0 },
		skill_levels = { TestJob = 5.0 },
		total_xp = 120.0,
		skill_xp = { TestJob = 120.0 }
	})
	set_component(agent_a, "Agent", {
		entity_id = agent_a,
		stamina = 100.0,
		state = "working"
	})

	-- Agent B: low skill (1.0) in TestJob via SkillLevels
	local agent_b = spawn_entity()
	set_component(agent_b, "SkillLevels", {
		skills = { TestJob = 1.0 },
		skill_levels = { TestJob = 1.0 },
		total_xp = 0.0,
		skill_xp = { TestJob = 0.0 }
	})
	set_component(agent_b, "Agent", {
		entity_id = agent_b,
		stamina = 100.0,
		state = "working"
	})

	-- Create two jobs of the same type
	local job_a_id = spawn_entity()
	assign_job(job_a_id, "TestJob", {
		state = "in_progress",
		progress = 0.0,
		required_progress = 100.0,
		category = "test",
		assigned_to = agent_a,
	})

	local job_b_id = spawn_entity()
	assign_job(job_b_id, "TestJob", {
		state = "in_progress",
		progress = 0.0,
		required_progress = 100.0,
		category = "test",
		assigned_to = agent_b,
	})

	-- Advance both jobs once
	advance_job_state(job_a_id)
	advance_job_state(job_b_id)

	local job_a = get_job(job_a_id)
	local job_b = get_job(job_b_id)

	-- skill=5.0 => increment = 1.0 * 5.0 * (100/100) = 5.0
	-- skill=1.0 => increment = 1.0 * 1.0 * (100/100) = 1.0
	-- Verify: progress_skill5 > progress_skill1 * 4
	assert.is_true(job_a.progress > job_b.progress * 4,
		"Expected skill=5 progress > 4x skill=1 progress, got " .. job_a.progress .. " vs " .. job_b.progress)
end

return {
	test_advance_job_progress = test_advance_job_progress,
	test_skill_multiplier_regression = test_skill_multiplier_regression,
}
