#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::JobSystem;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;
use world_helper::make_test_world;

/// Test pausing and resuming a job: progress halts while paused and continues after resume.
#[test]
fn test_job_can_be_paused_and_resumed() {
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Setup agent and job
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "status": "pending",
                "phase": "pending",
                "category": "mining",
                "target_position": {
                    "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
                },
                "progress": 0.0
            }),
        )
        .unwrap();

    // Assign job
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    world.register_system(JobSystem::new());

    // Tick once: job should start progressing
    world.run_system("JobSystem", None).unwrap();
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    let progress_after_1 = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(progress_after_1 > 0.0);

    // Pause job
    job["status"] = json!("paused");
    job["phase"] = json!("paused");
    world.set_component(job_id, "Job", job.clone()).unwrap();

    // Tick: progress should not advance
    for _ in 0..3 {
        world.run_system("JobSystem", None).unwrap();
    }
    let job = world.get_component(job_id, "Job").unwrap();
    let progress_while_paused = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert_eq!(
        progress_after_1, progress_while_paused,
        "Progress advanced while paused"
    );

    // Resume job
    let mut job = job.clone();
    job["status"] = json!("in_progress");
    job["phase"] = json!("in_progress");
    world.set_component(job_id, "Job", job).unwrap();

    // Tick: progress should resume
    let mut resumed = false;
    for _ in 0..10 {
        world.run_system("JobSystem", None).unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("status") == Some(&json!("complete")) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "Job did not complete after resume");
}

/// Test that an interrupted job can be resumed by another agent.
#[test]
fn test_job_is_interrupted_and_resumed_by_another_agent() {
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Setup agents and job
    let agent1 = world.spawn_entity();
    let agent2 = world.spawn_entity();
    world
        .set_component(
            agent1,
            "Agent",
            json!({
                "entity_id": agent1,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent2,
            "Agent",
            json!({
                "entity_id": agent2,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent1,
            "Position",
            json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();
    world
        .set_component(
            agent2,
            "Position",
            json!({
                "pos": { "Square": { "x": 1, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "status": "pending",
                "phase": "pending",
                "category": "mining",
                "target_position": {
                    "pos": { "Square": { "x": 2, "y": 0, "z": 0 } }
                },
                "progress": 0.0
            }),
        )
        .unwrap();

    // Assign job to agent1
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assert_eq!(
        job_board.claim_job(agent1, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    world.register_system(JobSystem::new());

    // Tick: agent1 starts job
    world.run_system("JobSystem", None).unwrap();
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    let progress_after_1 = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(progress_after_1 > 0.0);

    // Interrupt job (simulate agent1 unavailable)
    job["status"] = json!("interrupted");
    job["phase"] = json!("interrupted");
    job.as_object_mut().unwrap().remove("assigned_to");
    world.set_component(job_id, "Job", job.clone()).unwrap();

    // Assign job to agent2
    job_board.update(&world);
    assert_eq!(
        job_board.claim_job(agent2, &mut world, 1),
        JobAssignmentResult::Assigned(job_id)
    );

    // Tick: agent2 should resume and complete the job
    let mut resumed = false;
    for _ in 0..10 {
        world.run_system("JobSystem", None).unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("status") == Some(&json!("complete")) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "Job was not resumed and completed by agent2");
}

/// Test that world conditions affect job progress (e.g., "hazard" slows progress).
#[test]
fn test_job_progression_affected_by_world_conditions() {
    let mut world = make_test_world();
    world.current_mode = "colony".to_string();

    // Setup agent and job
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "state": "idle"
            }),
        )
        .unwrap();
    world
        .set_component(
            agent_id,
            "Position",
            json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "status": "pending",
                "phase": "pending",
                "category": "mining",
                "target_position": {
                    "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
                },
                "progress": 0.0
            }),
        )
        .unwrap();

    // Simulate a hazardous world condition by setting a global resource or flag
    world
        .set_component(
            0,
            "Hazard",
            json!({"active": true, "slowdown_factor": 0.25}),
        )
        .unwrap();

    // Register a custom job handler that checks for hazard and slows progress
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", |world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let hazard = world.get_component(0, "Hazard");
                let slowdown = hazard
                    .and_then(|hz| hz.get("slowdown_factor").and_then(|v| v.as_f64()))
                    .unwrap_or(1.0);
                progress += 1.0 * slowdown;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["status"] = json!("complete");
                } else {
                    job["status"] = json!("in_progress");
                }
                job
            });
    }

    world.register_system(JobSystem::new());

    // Tick: progress should be slow due to hazard
    let mut ticks = 0;
    loop {
        world.run_system("JobSystem", None).unwrap();
        ticks += 1;
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("status") == Some(&json!("complete")) {
            break;
        }
        assert!(ticks < 20, "Job did not complete in reasonable time");
    }
    assert!(ticks > 3, "Hazard did not slow job progress as expected");
}
