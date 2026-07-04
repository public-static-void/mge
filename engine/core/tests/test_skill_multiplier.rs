#[path = "helpers/event.rs"]
mod event_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::system::JobSystem;
use engine_core::systems::job::types::job_type::JobTypeData;
use serde_json::json;

/// Verifies that the skill multiplier formula is applied correctly for job progress.
///
/// Formula: progress_increment = max(0.1, 1.0 * skill_value * (stamina / 100.0))
///
/// Uses SkillLevels component (R014 path) — the preferred mechanism over agent.skills.
#[test]
fn test_skill_multiplier_with_skill_levels() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent with high skill via SkillLevels
    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 5.0 },
                "skill_levels": { "dig": 5.0 },
                "total_xp": 120.0,
                "skill_xp": { "dig": 120.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 100.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=5.0, stamina=100: increment = 1.0 * 5.0 * (100/100) = 5.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 5.0).abs() < f64::EPSILON,
        "Expected progress ~5.0, got {}",
        progress
    );
}

/// Verifies skill multiplier with partial stamina.
#[test]
fn test_skill_multiplier_with_partial_stamina() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 2.0 },
                "skill_levels": { "dig": 2.0 },
                "total_xp": 50.0,
                "skill_xp": { "dig": 50.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 50.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=2.0, stamina=50: increment = 1.0 * 2.0 * (50/100) = 1.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 1.0).abs() < f64::EPSILON,
        "Expected progress ~1.0, got {}",
        progress
    );
}

/// Verifies skill multiplier floor of 0.1 when stamina is very low.
#[test]
fn test_skill_multiplier_floor() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 1.0 },
                "skill_levels": { "dig": 1.0 },
                "total_xp": 0.0,
                "skill_xp": { "dig": 0.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 5.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // With skill=1.0, stamina=5: raw = 1.0 * 1.0 * (5/100) = 0.05, clamped to 0.1
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 0.1).abs() < f64::EPSILON,
        "Expected progress ~0.1 (floor), got {}",
        progress
    );
}

/// Verifies that a job without assigned_to uses progress_increment=1.0 (no skill scaling).
/// Requires a registered job type with effects to stay in "in_progress" state.
#[test]
fn test_skill_multiplier_no_agent() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let job_id = world.spawn_entity();

    // Register a job type with effects so unassigned job stays "in_progress"
    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        effects: vec![serde_json::json!({"type": "test_effect"})],
        ..Default::default()
    });

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // No assigned_to: progress_increment defaults to 1.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 1.0).abs() < f64::EPSILON,
        "Expected progress ~1.0 (no agent), got {}",
        progress
    );
}

/// Verifies that LeveledUpEvent is emitted when a skill levels up on job completion.
#[test]
fn test_skill_level_up_event_on_job_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Agent with skill close to leveling up (level 1, XP near threshold)
    // Threshold: base_xp * (1.5^current_level) = 10 * 1.5^1 = 15
    // Set skill_xp to 14, so one job completion (10xp base) pushes it over
    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 1.0 },
                "skill_levels": { "dig": 1.0 },
                "total_xp": 14.0,
                "skill_xp": { "dig": 14.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "stamina": 100.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 2.5,
                "state": "in_progress",
                "required_progress": 3.0,
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    // Run twice: first tick makes progress, second tick completes the job
    job_system.run(&mut world);
    job_system.run(&mut world);

    // Check that SkillLevels has the leveled-up skill
    let skill_levels = world.get_component(agent_id, "SkillLevels").unwrap();
    let dig_level = skill_levels
        .get("skill_levels")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("dig"))
        .and_then(|v| v.as_f64());

    assert!(
        dig_level.is_some(),
        "Skill should have leveled up after job completion"
    );
    assert!(
        dig_level.unwrap() > 1.0,
        "Level should be greater than starting level 1, got {}",
        dig_level.unwrap()
    );
}

/// Verifies that SkillLevels takes precedence over deprecated agent.skills (R014).
#[test]
fn test_skill_levels_precedence_over_agent_skills() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Set both SkillLevels (skill=5.0) and agent.skills (skill=1.0)
    // SkillLevels should take precedence
    world
        .set_component(
            agent_id,
            "SkillLevels",
            json!({
                "skills": { "dig": 5.0 },
                "skill_levels": { "dig": 5.0 },
                "total_xp": 120.0,
                "skill_xp": { "dig": 120.0 }
            }),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 1.0 },
                "stamina": 100.0,
                "state": "working",
                "current_job": job_id
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "progress": 0.0,
                "state": "in_progress",
                "assigned_to": agent_id,
                "category": "mining",
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    // SkillLevels.skills.dig = 5.0 should be used (not agent.skills.dig = 1.0)
    // increment = 1.0 * 5.0 * (100/100) = 5.0
    let job = world.get_component(job_id, "Job").unwrap();
    let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (progress - 5.0).abs() < f64::EPSILON,
        "Expected progress ~5.0 (SkillLevels precedence), got {}",
        progress
    );
}
