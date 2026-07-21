#[path = "helpers/world.rs"]
mod world_helper;

#[path = "helpers/resource.rs"]
mod resource_helper;
use resource_helper::ResourceTestHelpers;

#[path = "helpers/agent.rs"]
mod agent_helper;
use agent_helper::AgentTestHelpers;

#[path = "helpers/test_tick.rs"]
mod test_tick_helper;
use test_tick_helper::run_until;

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::system::events::{
    emit_job_event, init_job_event_logger, load_job_event_log, replay_job_event_log,
    save_job_event_log,
};
use engine_core::systems::job::{JobLogicKind, JobSystem, JobTypeData, assign_jobs};
use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};

const MAX_TICKS: usize = 16;

// --- Section: System ---

#[test]
fn test_register_job_schema_and_assign_job_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "haul_resource",
        "target": 42,
        "state": "pending",
        "progress": 0.0,
        "category": "hauling"
    });
    assert!(world.set_component(eid, "Job", job_val.clone()).is_ok());

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("job_type").unwrap(), "haul_resource");
    assert_eq!(job.get("state").unwrap(), "pending");
}

#[test]
fn test_query_entities_with_job_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();
    let eid = world.spawn_entity();

    let job_val = json!({
        "id": eid,
        "job_type": "build_structure",
        "state": "pending",
        "category": "construction"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    let with_job = world.get_entities_with_component("Job");
    assert_eq!(with_job, vec![eid]);
}

#[test]
fn test_job_system_advances_progress_and_completes_job() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Spawn agent and assign minimal Agent component
    let agent_eid = world.spawn_entity();
    let agent_component = serde_json::json!({
        "entity_id": agent_eid,
        "state": "idle",
        "skills": {
            "test_job": 1.0
        },
        "stamina": 100.0
    });
    assert!(
        world
            .set_component(agent_eid, "Agent", agent_component)
            .is_ok()
    );

    let eid = world.spawn_entity();
    let job_val = serde_json::json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "assigned_to": agent_eid,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val).unwrap();

    world.register_system(JobSystem);

    for _ in 0..4 {
        world.run_system("JobSystem").unwrap();
    }

    let job = world.get_component(eid, "Job").unwrap();
    assert_eq!(job.get("state").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 3.0);
}

#[test]
fn test_job_system_emits_event_on_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Spawn agent and assign minimal Agent component
    let agent_eid = world.spawn_entity();
    let agent_component = serde_json::json!({
        "entity_id": agent_eid,
        "state": "idle",
        "skills": {
            "test_job": 1.0
        },
        "stamina": 100.0
    });
    assert!(
        world
            .set_component(agent_eid, "Agent", agent_component)
            .is_ok()
    );

    let eid = world.spawn_entity();
    let job_val = serde_json::json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "assigned_to": agent_eid,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val).unwrap();

    world.register_system(JobSystem);

    for _ in 0..6 {
        world.run_system("JobSystem").unwrap();
    }

    world.update_event_buses::<serde_json::Value>();

    let bus = world
        .get_event_bus::<serde_json::Value>("job_completed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    assert!(!events.is_empty(), "No job_completed events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64)
    });
    assert!(found, "No job_completed event for our entity");
}

#[test]
fn test_job_system_emits_event_on_failure() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "test_job",
        "state": "pending",
        "progress": 0.0,
        "should_fail": true,
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val.clone()).unwrap();

    world.register_system(JobSystem);

    for _ in 0..2 {
        world.run_system("JobSystem").unwrap();
    }
    world.update_event_buses::<serde_json::Value>();

    let bus = world
        .get_event_bus::<serde_json::Value>("job_failed")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();

    assert!(!events.is_empty(), "No job_failed events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(eid as u64)
    });
    assert!(found, "No job_failed event for our entity");
}

#[test]
fn test_job_system_uses_custom_job_type_logic() {
    engine_core::systems::job::system::events::init_job_event_logger();
    use serde_json::json;

    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    {
        let mut reg = world.job_handler_registry.lock().unwrap();
        reg.register_handler("fast_job", |_world, _agent_id, _job_id, job| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) + 10.0;
            job["progress"] = serde_json::json!(progress);
            if progress >= 10.0 {
                job["state"] = serde_json::json!("complete");
            } else {
                job["state"] = serde_json::json!("in_progress");
            }
            job
        });
    }

    // Spawn agent and assign minimal Agent component
    let agent_eid = world.spawn_entity();
    let agent_component = json!({
        "entity_id": agent_eid,
        "state": "idle",
        "skills": {
            "fast_job": 1.0
        },
        "stamina": 100.0
    });
    assert!(
        world
            .set_component(agent_eid, "Agent", agent_component)
            .is_ok()
    );

    let eid = world.spawn_entity();
    let job_val = json!({
        "id": eid,
        "job_type": "fast_job",
        "state": "pending",
        "progress": 0.0,
        "assigned_to": agent_eid,
        "resource_requirements": [],
        "resource_outputs": [],
        "children": [],
        "dependencies": [],
        "category": "testing"
    });
    world.set_component(eid, "Job", job_val).unwrap();

    let mut job_system = JobSystem;
    for _ in 0..2 {
        engine_core::ecs::system::System::run(&mut job_system, &mut world);
    }

    let job = world.get_component(eid, "Job").unwrap();

    assert_eq!(job.get("state").unwrap(), "complete");
    assert!(job.get("progress").unwrap().as_f64().unwrap() >= 10.0);
}

#[test]
fn test_job_assignment_is_recorded_and_queryable() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let worker_eid = world.spawn_entity();
    let job_eid = world.spawn_entity();
    let job_val = json!({
        "id": job_eid,
        "job_type": "dig_tunnel",
        "state": "pending",
        "assigned_to": worker_eid,
        "category": "mining"
    });
    world
        .set_component(job_eid, "Job", job_val.clone())
        .unwrap();

    let job = world.get_component(job_eid, "Job").unwrap();
    assert_eq!(
        job.get("assigned_to").unwrap().as_u64().unwrap(),
        worker_eid as u64,
        "Job should be assigned to worker"
    );
}

// --- Section: Lifecycle ---

/// Tests that an agent is assigned a job, completes it, and becomes idle.
#[test]
fn test_agent_state_and_job_completion() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 5.0 },
                "preferences": { "dig": 2.0 },
                "state": "idle"
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
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    let job = world.get_component(job_id, "Job").unwrap();

    assert_eq!(agent["current_job"], job_id, "Agent should be assigned job");
    assert_eq!(agent["state"], "working", "Agent should be working");
    assert_eq!(
        job["assigned_to"], agent_id,
        "Job should be assigned to agent"
    );

    // Let the system process the job to completion
    let mut job_system = JobSystem;
    for _ in 0..5 {
        job_system.run(&mut world);
        let job = world.get_component(job_id, "Job").unwrap();
        if job["state"] == "complete" {
            break;
        }
    }

    let agent = world.get_component(agent_id, "Agent").unwrap();
    assert_eq!(
        agent.get("current_job"),
        Some(&serde_json::Value::Null),
        "Agent should have no current job after completion"
    );
    assert_eq!(
        agent["state"], "idle",
        "Agent should be idle after completion"
    );
}

/// Tests that a higher-priority job preempts a lower-priority job, and that the agent is reassigned after completion.
#[test]
fn test_job_preemption_and_reassignment() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            json!({
                "entity_id": agent_id,
                "skills": { "dig": 1.0, "build": 1.0 },
                "preferences": { "dig": 2.0 },
                "state": "idle"
            }),
        )
        .unwrap();

    // Low priority job assigned first
    let job100 = world.spawn_entity();
    world
        .set_component(
            job100,
            "Job",
            json!({
                "id": job100,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "required_progress": 10.0
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);

    assign_jobs(&mut world, &mut job_board, 0, &[]);

    {
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job = world.get_component(job100, "Job").unwrap();
        assert_eq!(
            agent["current_job"], job100,
            "Agent should be assigned job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job["assigned_to"], agent_id,
            "Job 100 should be assigned to agent"
        );
    }

    // Now, a higher-priority job appears
    let job200 = world.spawn_entity();
    world
        .set_component(
            job200,
            "Job",
            json!({
                "id": job200,
                "job_type": "dig",
                "state":"pending",
                "priority": 10,
                "category": "mining"
            }),
        )
        .unwrap();

    // Run the job system and assign_jobs in a loop until preemption occurs
    let mut preempted = false;
    for _ in 0..10 {
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);

        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();
        let job200_obj = world.get_component(job200, "Job").unwrap();

        if agent["current_job"] == job200 {
            preempted = true;
            // The agent should now be working on job 200 (preemption)
            assert_eq!(
                agent["current_job"], job200,
                "Agent should be assigned job 200 (preemption)"
            );
            assert_eq!(agent["state"], "working", "Agent should be working");
            assert_eq!(
                job200_obj["assigned_to"], agent_id,
                "Job 200 should be assigned to agent"
            );

            // Job 100 should be unassigned and set to pending
            assert!(
                job100_obj.get("assigned_to") == Some(&serde_json::Value::Null),
                "Job 100 should be unassigned"
            );
            assert_eq!(job100_obj["state"], "pending", "Job 100 should be pending");
            break;
        }

        let mut job_system = JobSystem;
        job_system.run(&mut world);
    }

    assert!(preempted, "Agent was never preempted to job 200");

    // Simulate job 200 completion and agent becoming idle
    {
        let mut job = world.get_component(job200, "Job").unwrap().clone();
        job["state"] = json!("complete");
        world.set_component(job200, "Job", job).unwrap();
    }

    {
        let mut agent = world.get_component(agent_id, "Agent").unwrap().clone();
        agent.as_object_mut().unwrap().remove("current_job");
        agent["state"] = json!("idle");
        world.set_component(agent_id, "Agent", agent).unwrap();
    }

    // --- Repeatedly assign jobs and run the job system until job 100 is reassigned ---
    loop {
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();
        if agent["current_job"] == job100 && job100_obj["assigned_to"] == agent_id {
            break;
        }
        let mut job_system = JobSystem;
        job_system.run(&mut world);
    }

    {
        let agent = world.get_component(agent_id, "Agent").unwrap();
        let job100_obj = world.get_component(job100, "Job").unwrap();

        // The agent should now be reassigned to job 100
        assert_eq!(
            agent["current_job"], job100,
            "Agent should be reassigned to job 100"
        );
        assert_eq!(agent["state"], "working", "Agent should be working");
        assert_eq!(
            job100_obj["assigned_to"], agent_id,
            "Job 100 should be assigned to agent"
        );
    }
}

// --- Section: Lifecycle Control ---

/// Test pausing and resuming a job: progress halts while paused and continues after resume.
#[test]
fn test_job_can_be_paused_and_resumed() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Register a handler for "dig" jobs that respects pausing
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", move |_world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                progress += 1.0;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
                job
            });
    }

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
                "state": "pending",
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
    job_board.update(&world, 0, &[]);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    world.register_system(JobSystem::new());

    // Tick until progress starts
    let mut progress_after_1 = 0.0;
    for _ in 0..10 {
        world.run_system("JobSystem").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        progress_after_1 = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if progress_after_1 > 0.0 {
            break;
        }
    }
    assert!(
        progress_after_1 > 0.0,
        "Job did not start progressing after first tick"
    );

    // Pause job
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    job["state"] = json!("paused");
    world.set_component(job_id, "Job", job.clone()).unwrap();

    // Tick: progress should not advance
    for _ in 0..3 {
        world.run_system("JobSystem").unwrap();
    }
    let job = world.get_component(job_id, "Job").unwrap();
    let progress_while_paused = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert_eq!(
        progress_after_1, progress_while_paused,
        "Progress advanced while paused"
    );

    // Resume job
    let mut job = job.clone();
    job["state"] = json!("in_progress");
    world.set_component(job_id, "Job", job).unwrap();

    // Tick: progress should resume
    let mut resumed = false;
    for _ in 0..10 {
        world.run_system("JobSystem").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("state") == Some(&json!("complete")) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "Job did not complete after resume");
}

/// Test that an interrupted job can be resumed by another agent.
#[test]
fn test_job_is_interrupted_and_resumed_by_another_agent() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    // Register a handler for "dig" jobs that respects pausing/interruption
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", move |_world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                progress += 1.0;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
                job
            });
    }

    // Map setup: 3-square-wide walkable map
    let map_json = serde_json::json!({
        "topology": "square",
        "width": 3,
        "height": 1,
        "z_levels": 1,
        "cells": [
            { "x": 0, "y": 0, "z": 0, "walkable": true },
            { "x": 1, "y": 0, "z": 0, "walkable": true },
            { "x": 2, "y": 0, "z": 0, "walkable": true }
        ]
    });
    world.apply_generated_map(&map_json).unwrap();

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
                "state": "pending",
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
    job_board.update(&world, 0, &[]);
    assert_eq!(
        job_board.claim_job(agent1, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    world.register_system(JobSystem::new());
    use engine_core::systems::movement_system::MovementSystem;
    world.register_system(MovementSystem);

    // Tick: agent1 starts job (simulate movement if needed)
    let mut progressed = false;
    for _ in 0..40 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0 {
            progressed = true;
            break;
        }
    }
    assert!(
        progressed,
        "Job did not start progressing after being assigned to agent1"
    );

    // Interrupt job (simulate agent1 unavailable)
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    job["state"] = json!("interrupted");
    job.as_object_mut().unwrap().remove("assigned_to");
    world.set_component(job_id, "Job", job.clone()).unwrap();

    // Assign job to agent2
    job_board.update(&world, 0, &[]);
    assert_eq!(
        job_board.claim_job(agent2, &mut world, 1),
        JobAssignmentResult::Assigned(job_id)
    );

    // Tick: agent2 should resume and complete the job (simulate movement)
    let mut resumed = false;
    for _ in 0..80 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("state") == Some(&json!("complete")) {
            resumed = true;
            break;
        }
    }
    assert!(resumed, "Job was not resumed and completed by agent2");
}

/// Test that world conditions affect job progress (e.g., "hazard" slows progress).
#[test]
fn test_job_progression_affected_by_world_conditions() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
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
                "state": "pending",
                "category": "mining",
                "target_position": {
                    "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
                },
                "progress": 0.0
            }),
        )
        .unwrap();

    // Simulate a hazardous world condition by setting a global resource or flag
    let hazard_id = world.spawn_entity();
    world
        .set_component(
            hazard_id,
            "Hazard",
            json!({"active": true, "slowdown_factor": 0.25}),
        )
        .unwrap();

    // Register a custom job handler that checks for hazard and slows progress, and respects pausing
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", move |world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                // Find any Hazard component (use first found)
                let slowdown = world
                    .components
                    .get("Hazard")
                    .and_then(|map| map.values().next())
                    .and_then(|hz| hz.get("slowdown_factor").and_then(|v| v.as_f64()))
                    .unwrap_or(1.0);
                progress += 1.0 * slowdown;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
                job
            });
    }

    world.register_system(JobSystem::new());

    // Assign job to agent BEFORE ticking
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assert_eq!(
        job_board.claim_job(agent_id, &mut world, 0),
        JobAssignmentResult::Assigned(job_id)
    );

    // Tick: progress should be slow due to hazard
    let mut ticks = 0;
    loop {
        world.run_system("JobSystem").unwrap();
        ticks += 1;
        let job = world.get_component(job_id, "Job").unwrap();
        if job.get("state") == Some(&json!("complete")) {
            break;
        }
        assert!(ticks < 20, "Job did not complete in reasonable time");
    }
    assert!(ticks > 3, "Hazard did not slow job progress as expected");
}

// --- Section: Progress Events ---

#[test]
fn test_job_progressed_event_emitted_on_progress_change() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Agent and job setup
    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();
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
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();

    // Run job system for several ticks, capturing progress events
    let mut all_events = Vec::new();
    for _ in 0..5 {
        job_system.run(&mut world);
        world.update_event_buses::<serde_json::Value>();
        let bus = world
            .get_event_bus::<serde_json::Value>("job_progressed")
            .unwrap();
        let mut reader = engine_core::ecs::event::EventReader::default();
        let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
        all_events.extend(events);
    }

    // There should be at least one progress event, and all should have correct entity and progress
    assert!(!all_events.is_empty(), "No job_progressed events emitted");
    for event in &all_events {
        assert_eq!(
            event.get("entity").and_then(|v| v.as_u64()),
            Some(job_id as u64),
            "Event should reference job entity"
        );
        assert!(
            event.get("progress").is_some(),
            "Event should have progress"
        );
        assert!(event.get("state").is_some(), "Event should have state");
    }

    // There should be no duplicate events for the same progress value
    let mut seen = Vec::new();
    for event in &all_events {
        let progress = event.get("progress").and_then(|v| v.as_f64()).unwrap();
        assert!(
            !seen.contains(&progress),
            "Duplicate progress event for value {progress}"
        );
        seen.push(progress);
    }
}

#[test]
fn test_job_progressed_event_emitted_for_custom_handler() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a custom handler that sets progress in two steps
    world.register_job_handler(
        "twostep",
        |_world, _agent_id, _job_id, job: &serde_json::Value| {
            let mut job = job.clone();
            let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if progress < 1.0 {
                job["progress"] = serde_json::json!(1.0);
                job["state"] = serde_json::json!("in_progress");
            } else {
                job["progress"] = serde_json::json!(2.0);
                job["state"] = serde_json::json!("complete");
            }
            job
        },
    );

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();
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
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "twostep",
                "state": "pending",
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();

    let mut all_events = Vec::new();
    for _ in 0..3 {
        job_system.run(&mut world);
        world.update_event_buses::<serde_json::Value>();
        let bus = world
            .get_event_bus::<serde_json::Value>("job_progressed")
            .unwrap();
        let mut reader = engine_core::ecs::event::EventReader::default();
        let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
        all_events.extend(events);
    }

    // Should have progress events for both 1.0 and 2.0
    let progresses: Vec<_> = all_events
        .iter()
        .map(|e| e.get("progress").and_then(|v| v.as_f64()).unwrap())
        .collect();
    assert!(
        progresses.contains(&1.0),
        "Should have progress event for 1.0"
    );
    assert!(
        progresses.contains(&2.0),
        "Should have progress event for 2.0"
    );
}

// --- Section: Cancellation ---

#[test]
fn test_job_cancellation_cleans_up_agent_and_emits_event() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a dummy effect handler (should not be called on cancel)
    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |_world, _eid, _effect| {
                panic!("Effect should not be processed on cancellation");
            });
        world.effect_processor_registry = Some(registry);
    }

    // Agent and job setup
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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let _agent = world.get_component(agent_id, "Agent").unwrap();
    let _job = world.get_component(job_id, "Job").unwrap();

    // Cancel the job (by setting state to "cancelled")
    let mut job = world.get_component(job_id, "Job").unwrap().clone();
    job["state"] = json!("cancelled");
    world.set_component(job_id, "Job", job).unwrap();

    // Run job system to process cancellation
    let mut job_system = JobSystem;
    job_system.run(&mut world);

    let agent = world.get_component(agent_id, "Agent").unwrap();
    let job = world.get_component(job_id, "Job").unwrap();

    // Agent should be idle and unassigned
    assert!(
        agent.get("current_job").is_none_or(|v| v.is_null()),
        "Agent should have no current job after cancellation"
    );
    assert_eq!(
        agent["state"], "idle",
        "Agent should be idle after cancellation"
    );

    // Job should be marked as cancelled
    assert_eq!(job["state"], "cancelled", "Job state should be 'cancelled'");

    // Event should be emitted
    world.update_event_buses::<serde_json::Value>();
    let bus = world
        .get_event_bus::<serde_json::Value>("job_cancelled")
        .unwrap();
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert!(
        events
            .iter()
            .any(|event| event.get("entity").and_then(|v| v.as_u64()) == Some(job_id as u64)),
        "Job cancelled event should be emitted for job"
    );
}

#[test]
fn test_job_effect_rollback_on_cancel() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a reversible effect handler
    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |world, eid, effect| {
                let to = effect.get("to").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": to }))
                    .unwrap();
            });
        registry
            .lock()
            .unwrap()
            .register_handler("UndoModifyTerrain", |world, eid, effect| {
                let from = effect.get("from").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": from }))
                    .unwrap();
            });
        world.effect_processor_registry = Some(registry);
    }

    // Set up initial terrain
    let entity_id = world.spawn_entity();
    world
        .set_component(entity_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    // Job with an effect
    world
        .set_component(
            entity_id,
            "Job",
            json!({
                "id": entity_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    // Simulate job type registry with effect
    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    // Assign and complete job normally: effect should apply
    {
        let mut job_board = JobBoard::default();
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);

        // Mark job as complete
        let mut job = world.get_component(entity_id, "Job").unwrap().clone();
        job["state"] = json!("complete");
        world.set_component(entity_id, "Job", job).unwrap();

        let _terrain = world.get_component(entity_id, "Terrain").unwrap();

        // Run system to apply effect
        let mut job_system = JobSystem;
        job_system.run(&mut world);

        let terrain = world.get_component(entity_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "tunnel",
            "Terrain should be tunnel after effect"
        );
    }

    // Reset for cancellation test
    world
        .set_component(entity_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            entity_id,
            "Job",
            json!({
                "id": entity_id,
                "job_type": "dig",
                "state": "cancelled",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let _terrain = world.get_component(entity_id, "Terrain").unwrap();

    // Run system: effect should not apply, and rollback (UndoModifyTerrain) should be called
    {
        let mut job_system = JobSystem;
        job_system.run(&mut world);

        let terrain = world.get_component(entity_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "rock",
            "Terrain should remain rock after cancellation"
        );
    }
}

#[test]
fn test_job_cancellation_releases_resources_and_cancels_children() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up stockpile with resources and a job that reserves them
    let stockpile_id = world.spawn_entity();
    world
        .set_component(
            stockpile_id,
            "Stockpile",
            json!({ "resources": { "wood": 10 } }),
        )
        .unwrap();

    let child_id = world.spawn_entity();
    let parent_id = world.spawn_entity();

    world
        .set_component(
            child_id,
            "Job",
            json!({
                "id": child_id,
                "job_type": "subtask",
                "state": "pending",
                "category": "construction"
            }),
        )
        .unwrap();

    world
        .set_component(
            parent_id,
            "Job",
            json!({
                "id": parent_id,
                "job_type": "build",
                "state": "pending",
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "reserved_resources": [{ "kind": "wood", "amount": 5 }],
                "reserved_stockpile": stockpile_id,
                "category": "construction",
                "children": [
                    {
                        "id": child_id,
                        "job_type": "subtask",
                        "state": "pending",
                        "category": "construction"
                    }
                ]
            }),
        )
        .unwrap();

    let _stockpile = world.get_component(stockpile_id, "Stockpile").unwrap();
    let _parent_job = world.get_component(parent_id, "Job").unwrap();

    // Cancel the parent job (by setting state to "cancelled")
    let mut job = world.get_component(parent_id, "Job").unwrap().clone();
    job["state"] = json!("cancelled");
    world.set_component(parent_id, "Job", job).unwrap();

    // Run job system to process cancellation
    let mut job_system = JobSystem;
    job_system.run(&mut world);

    let stockpile = world.get_component(stockpile_id, "Stockpile").unwrap();
    let parent_job = world.get_component(parent_id, "Job").unwrap();

    // Check that resources are released
    assert_eq!(
        stockpile["resources"]["wood"], 10,
        "Resources should be released back to stockpile"
    );

    // Check that child job is cancelled
    let child_id = parent_job
        .get("children")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|child| child.get("id"))
        .and_then(|id| id.as_u64())
        .unwrap() as u32;
    let child_job = world.get_component(child_id, "Job").unwrap();
    assert_eq!(
        child_job["state"], "cancelled",
        "Child job should be cancelled"
    );
}

// --- Section: Rollback and Interruption ---

#[test]
fn test_partial_effect_rollback_on_cancel_and_interruption() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register minimal schemas for test components in "colony" mode
    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &serde_json::json!({
                "title": "Step",
                "type": "object",
                "properties": { "value": { "type": "integer" } },
                "required": ["value"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    world
        .registry
        .lock()
        .unwrap()
        .register_external_schema_from_json(
            &serde_json::json!({
                "title": "Agent",
                "type": "object",
                "properties": {
                    "entity_id": { "type": "integer" },
                    "state": { "type": "string" }
                },
                "required": ["entity_id", "state"],
                "modes": ["colony"]
            })
            .to_string(),
        )
        .unwrap();

    // Register effect handlers and their undo handlers
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step1", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 1}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in Step1 effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_handler("Step2", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 2}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in Step2 effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep1", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 0}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in UndoStep1 undo effect");
            }
        });
    world
        .effect_processor_registry
        .as_ref()
        .expect("EffectProcessorRegistry missing")
        .lock()
        .unwrap()
        .register_undo_handler("UndoStep2", |world, eid, _effect| {
            let res = world.set_component(eid, "Step", serde_json::json!({"value": 1}));
            if res.is_err() {
                println!("Warning: Failed to set Step component in UndoStep2 undo effect");
            }
        });

    // Register the JobSystem
    world.systems.register_system(JobSystem::new());

    // Register job type with two sequential effects
    let job_type_data = JobTypeData {
        name: "RollbackJob".to_string(),
        requirements: vec![],
        duration: Some(1.0),
        effects: vec![
            serde_json::json!({ "action": "Step1" }),
            serde_json::json!({ "action": "Step2" }),
        ],
    };

    // Job handler closure
    let job_handler = |world: &mut engine_core::ecs::world::World,
                       _assigned_to: u32,
                       job_id: u32,
                       job: &serde_json::Value| {
        let mut job_map = job.as_object().cloned().unwrap_or_default();
        let job_type = job.get("job_type").and_then(|v| v.as_str()).unwrap_or("");

        engine_core::systems::job::system::effects::process_job_effects(
            world,
            job_id,
            job_type,
            &mut job_map,
            false,
        );
        serde_json::Value::Object(job_map)
    };

    world
        .job_types
        .register(job_type_data, JobLogicKind::Native(job_handler));
    world
        .job_handler_registry
        .lock()
        .unwrap()
        .register_handler("RollbackJob", job_handler);

    // Create an entity and assign the job with required_progress=2.0
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Job",
            serde_json::json!({
                "id": eid,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "required_progress": 2.0,
                "category": "test"
            }),
        )
        .unwrap();

    // Run the job system once: should apply first effect only (Step1)
    world.run_system("JobSystem").unwrap();

    // Check job component presence
    let _job_state = world
        .get_component(eid, "Job")
        .expect("Job component missing after first tick");

    // Check Step component set by first effect handler
    let step = world
        .get_component(eid, "Step")
        .expect("Step component missing after first tick");
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Mark job as cancelled before second effect
    let mut job = world
        .get_component(eid, "Job")
        .expect("Job component missing for cancellation")
        .clone();
    job["state"] = serde_json::json!("cancelled");
    world.set_component(eid, "Job", job).unwrap();

    // Run the job system again: should rollback the first effect only
    world.run_system("JobSystem").unwrap();

    // Check Step component after rollback undo on cancellation
    let step = world
        .get_component(eid, "Step")
        .expect("Step component missing after cancellation rollback");
    assert_eq!(
        step["value"], 0,
        "First effect should be rolled back on cancel"
    );

    // Now test interruption (simulate agent death/unavailable)
    let agent_id = world.spawn_entity();
    world
        .set_component(
            agent_id,
            "Agent",
            serde_json::json!({
                "entity_id": agent_id,
                "state": "working"
            }),
        )
        .unwrap();

    let eid2 = world.spawn_entity();
    world
        .set_component(
            eid2,
            "Job",
            serde_json::json!({
                "id": eid2,
                "job_type": "RollbackJob",
                "state": "pending",
                "progress": 0.0,
                "required_progress": 2.0,
                "category": "test",
                "assigned_to": agent_id
            }),
        )
        .unwrap();

    // Run the job system once: first effect applied (Step1)
    world.run_system("JobSystem").unwrap();

    let _job_state2 = world
        .get_component(eid2, "Job")
        .expect("Job component missing after first tick of second job");

    let step = world
        .get_component(eid2, "Step")
        .expect("Step component missing after first tick of second job");
    assert_eq!(step["value"], 1, "First effect should be applied");

    // Simulate agent death/unavailability
    world.despawn_entity(agent_id);

    // Confirm job entity still exists before rollback
    let job2_exists = world.entity_exists(eid2);
    assert!(job2_exists, "Job entity should still exist before rollback");

    // Run the job system again: should detect missing agent and rollback
    world.run_system("JobSystem").unwrap();

    // Check Step component after rollback on interruption
    let step_after_interrupt = world
        .get_component(eid2, "Step")
        .expect("Step component missing after interruption rollback");
    assert_eq!(
        step_after_interrupt["value"], 0,
        "Effect should be rolled back on agent interruption"
    );
}

// --- Section: Events ---

#[test]
fn test_job_lifecycle_events_emitted() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Register a handler for "dig" jobs so the job can progress and complete
    {
        let registry = world.job_handler_registry.clone();
        registry
            .lock()
            .unwrap()
            .register_handler("dig", move |_world, _agent_id, _job_id, job| {
                let mut job = job.clone();
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if matches!(
                    state,
                    "failed" | "complete" | "cancelled" | "interrupted" | "paused"
                ) {
                    return job;
                }
                let mut progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                progress += 1.0;
                job["progress"] = json!(progress);
                if progress >= 3.0 {
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
                job
            });
    }

    // Agent and job setup
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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "progress": 0.0
            }),
        )
        .unwrap();

    // Assign job to agent
    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    // Advance event buses to make assigned event visible
    world.update_event_buses::<serde_json::Value>();

    // Collect assigned event after assignment and update
    let assigned_events = world.drain_events::<serde_json::Value>("job_assigned");

    // Accumulate all events over all ticks
    let mut completed_events: Vec<serde_json::Value> = Vec::new();
    let mut failed_events: Vec<serde_json::Value> = Vec::new();
    let mut cancelled_events: Vec<serde_json::Value> = Vec::new();
    let mut progress_events: Vec<serde_json::Value> = Vec::new();

    let mut job_system = JobSystem::new();
    for _ in 0..10 {
        // Increase to 10 ticks to guarantee completion
        job_system.run(&mut world);

        // Advance event buses after system run, before draining
        world.update_event_buses::<serde_json::Value>();

        completed_events.extend(world.drain_events::<serde_json::Value>("job_completed"));
        failed_events.extend(world.drain_events::<serde_json::Value>("job_failed"));
        cancelled_events.extend(world.drain_events::<serde_json::Value>("job_cancelled"));
        progress_events.extend(world.drain_events::<serde_json::Value>("job_progressed"));
    }

    // Check that job_assigned event was emitted
    assert!(
        assigned_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_assigned event for job"
    );
    // Check that job_completed event was emitted
    assert!(
        completed_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_completed event for job"
    );
    // Check that at least one progress event was emitted
    assert!(
        progress_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(job_id))),
        "No job_progressed event for job"
    );

    // Check event payloads are rich and consistent
    for event in assigned_events
        .iter()
        .chain(completed_events.iter())
        .chain(progress_events.iter())
    {
        assert!(event.get("entity").is_some());
        assert!(event.get("job_type").is_some());
        assert!(event.get("state").is_some());
    }
}

#[test]
fn test_job_cancel_and_failure_events() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Setup a job that will fail
    let fail_job_id = world.spawn_entity();
    world
        .set_component(
            fail_job_id,
            "Job",
            json!({
                "id": fail_job_id,
                "job_type": "failtest",
                "state": "pending",
                "should_fail": true,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

    // Setup a job that will be cancelled
    let cancel_job_id = world.spawn_entity();
    world
        .set_component(
            cancel_job_id,
            "Job",
            json!({
                "id": cancel_job_id,
                "job_type": "dig",
                "state": "cancelled",
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    let mut failed_events: Vec<serde_json::Value> = Vec::new();
    let mut cancelled_events: Vec<serde_json::Value> = Vec::new();

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world);

        // Advance event buses after system run, before draining
        world.update_event_buses::<serde_json::Value>();

        failed_events.extend(world.drain_events::<serde_json::Value>("job_failed"));
        cancelled_events.extend(world.drain_events::<serde_json::Value>("job_cancelled"));
    }

    assert!(
        failed_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(fail_job_id))),
        "No job_failed event for failed job"
    );
    assert!(
        cancelled_events
            .iter()
            .any(|e| e.get("entity") == Some(&json!(cancel_job_id))),
        "No job_cancelled event for cancelled job"
    );
}

#[test]
fn test_job_event_logging_and_replay() {
    // Clean up any previous test log
    let log_path = "test_job_event_log.json";
    let _ = fs::remove_file(log_path);

    // --- Original run: emit events and save log ---
    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));
    init_job_event_logger();
    let mut world = World::new(registry.clone());

    // Create a dummy job
    let job = json!({
        "id": 42,
        "job_type": "dig",
        "state": "pending",
        "priority": 5,
        "assigned_to": null
    });

    // Subscribe to job events and collect them
    let received_events = Arc::new(Mutex::new(Vec::new()));
    {
        let received_events = received_events.clone();
        world
            .event_buses
            .get_event_bus::<serde_json::Value>("job_assigned")
            .unwrap_or_else(|| {
                world.event_buses.register_event_bus(
                    "job_assigned".to_string(),
                    Arc::new(Mutex::new(engine_core::ecs::event::EventBus::<
                        serde_json::Value,
                    >::default())),
                );
                world
                    .event_buses
                    .get_event_bus::<serde_json::Value>("job_assigned")
                    .unwrap()
            })
            .lock()
            .unwrap()
            .subscribe(move |event| {
                received_events.lock().unwrap().push(event.clone());
            });
    }

    // Emit a job assignment event
    emit_job_event(&mut world, "job_assigned", &job, None);

    // Save the event log
    save_job_event_log(log_path).expect("Failed to save job event log");

    // --- Replay run: load log and replay into a new world ---
    let registry = Arc::new(Mutex::new(ComponentRegistry::default()));
    init_job_event_logger();
    let mut replayed_world = World::new(registry);

    // Set up a new event bus and collector for replayed events
    let replayed_events = Arc::new(Mutex::new(Vec::new()));
    {
        let replayed_events = replayed_events.clone();
        replayed_world.event_buses.register_event_bus(
            "job_assigned".to_string(),
            Arc::new(Mutex::new(engine_core::ecs::event::EventBus::<
                serde_json::Value,
            >::default())),
        );
        replayed_world
            .event_buses
            .get_event_bus::<serde_json::Value>("job_assigned")
            .unwrap()
            .lock()
            .unwrap()
            .subscribe(move |event| {
                replayed_events.lock().unwrap().push(event.clone());
            });
    }

    // Load and replay the log
    load_job_event_log(log_path).expect("Failed to load job event log");
    replay_job_event_log(&mut replayed_world);

    // --- Assert that the replayed event matches the original ---
    // When running alongside other tests, the shared event bus may contain
    // events from other tests. Verify our specific event is present.
    let replayed_events = replayed_events.lock().unwrap();
    assert!(!replayed_events.is_empty(), "No events replayed");
    let found = replayed_events.iter().any(|e| {
        e.get("entity") == Some(&json!(42))
            && e.get("job_type") == Some(&json!("dig"))
            && e.get("state") == Some(&json!("pending"))
            && e.get("priority") == Some(&json!(5))
    });
    assert!(found, "Replayed event with entity=42, job_type=dig not found");

    // Clean up
    let _ = fs::remove_file(log_path);
}

// --- Section: Progression ---

#[test]
fn test_job_progression_over_ticks() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let agent_id = world.spawn_entity();

    world
        .set_component(
            agent_id,
            "Position",
            serde_json::json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "target_position": { "pos": { "Square": { "x": 0, "y": 0, "z": 0 } } }
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world);
        let job = world.get_component(job_id, "Job").unwrap();
        let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let state = job.get("state").and_then(|v| v.as_str()).unwrap();
        if progress < 3.0 {
            assert_eq!(
                state, "in_progress",
                "Job should be in progress while progress < 3.0"
            );
        } else {
            assert_eq!(
                state, "complete",
                "Job should be complete when progress >= 3.0"
            );
            break;
        }
    }
    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(
        job.get("state").unwrap(),
        "complete",
        "Job should be complete after progression"
    );
}

#[test]
fn test_custom_job_handler_overrides_progression() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    {
        let registry = world.job_handler_registry.clone();
        registry.lock().unwrap().register_handler(
            "instant",
            |_world, _agent_id, _job_id, job: &serde_json::Value| {
                let mut job = job.clone();
                job["progress"] = serde_json::json!(42.0);
                job["state"] = serde_json::json!("complete");
                job
            },
        );
    }

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

    let job_id = world.spawn_entity();
    world
        .set_component(
            job_id,
            "Job",
            json!({
                "id": job_id,
                "job_type": "instant",
                "state": "pending",
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();

    let mut job_board = JobBoard::default();
    job_board.update(&world, 0, &[]);
    assign_jobs(&mut world, &mut job_board, 0, &[]);

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(
        job.get("progress").unwrap(),
        42.0,
        "Progress should be set by custom handler"
    );
    assert_eq!(
        job.get("state").unwrap(),
        "complete",
        "state should be set by custom handler"
    );
}

#[test]
fn test_effects_applied_only_on_completion_and_rolled_back_on_cancel() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    {
        let registry = world.effect_processor_registry.take().unwrap();
        registry
            .lock()
            .unwrap()
            .register_handler("ModifyTerrain", |world, eid, effect| {
                let to = effect.get("to").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": to }))
                    .unwrap();
            });
        registry
            .lock()
            .unwrap()
            .register_handler("UndoModifyTerrain", |world, eid, effect| {
                let from = effect.get("from").and_then(|v| v.as_str()).unwrap();
                world
                    .set_component(eid, "Terrain", json!({ "kind": from }))
                    .unwrap();
            });
        world.effect_processor_registry = Some(registry);
    }

    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    let terrain_id = world.spawn_entity();
    world
        .set_component(terrain_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    // Ensure there is a valid AGENT able to perform the job
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
            serde_json::json!({
                "pos": { "Square": { "x": 0, "y": 0, "z": 0 } }
            }),
        )
        .unwrap();

    // Assign a position to the job, matching the agent's
    world
        .set_component(
            terrain_id,
            "Job",
            json!({
                "id": terrain_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "target_position": { "pos": { "Square": { "x": 0, "y": 0, "z": 0 } } }
            }),
        )
        .unwrap();

    {
        let mut job_board = JobBoard::default();
        job_board.update(&world, 0, &[]);
        assign_jobs(&mut world, &mut job_board, 0, &[]);

        let mut job_system = JobSystem::new();
        for _ in 0..20 {
            job_system.run(&mut world);
            let job = world.get_component(terrain_id, "Job").unwrap();
            if job.get("state") == Some(&serde_json::json!("complete")) {
                break;
            }
        }

        let terrain = world.get_component(terrain_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "tunnel",
            "Terrain should change to tunnel after job completion"
        );
    }

    world
        .set_component(terrain_id, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    world
        .set_component(
            terrain_id,
            "Job",
            json!({
                "id": terrain_id,
                "job_type": "dig",
                "state": "cancelled",
                "priority": 1,
                "category": "mining",
                "target_position": { "pos": { "Square": { "x": 0, "y": 0, "z": 0 } } }
            }),
        )
        .unwrap();

    {
        let mut job_system = JobSystem::new();
        job_system.run(&mut world);

        let terrain = world.get_component(terrain_id, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "rock",
            "Terrain should remain rock after job cancellation"
        );
    }
}

#[test]
fn test_agent_moves_to_job_site_before_progress() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up a 3x3 grid map with all cells and neighbors for pathfinding
    use engine_core::map::{Map, SquareGridMap};
    let mut sq_map = SquareGridMap::new();
    for x in 0..=2 {
        for y in 0..=2 {
            sq_map.add_cell(x, y, 0);
        }
    }
    // Add neighbors (4-way connectivity)
    for x in 0..=2 {
        for y in 0..=2 {
            let dirs = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for (dx, dy) in dirs {
                let nx = x + dx;
                let ny = y + dy;
                if (0..=2).contains(&nx) && (0..=2).contains(&ny) {
                    sq_map.add_neighbor((x, y, 0), (nx, ny, 0));
                }
            }
        }
    }
    world.map = Some(Map::new(Box::new(sq_map)));

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Register agent and job before assignment
    world
        .set_component(
            agent_id,
            "Position",
            serde_json::to_value(engine_core::ecs::components::position::PositionComponent {
                pos: engine_core::ecs::components::position::Position::Square { x: 0, y: 0, z: 0 },
            })
            .unwrap(),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            serde_json::json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": [],
                "job_queue": [],
                "move_path": [],
                "carried_resources": []
            }),
        )
        .unwrap();

    world
        .set_component(
            job_id,
            "Job",
            serde_json::json!({
                "id": job_id,
                "job_type":"dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "target_position": {
                    "pos": {
                        "Square": { "x": 2, "y": 2, "z": 0 }
                    }
                },
                "resource_requirements": [
                    { "kind": "dirt", "amount": 0 }
                ]
            }),
        )
        .unwrap();

    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    let mut job_board = engine_core::systems::job::job_board::JobBoard::default();
    job_board.update(&world, 0, &[]);
    engine_core::systems::job::assign_jobs(&mut world, &mut job_board, 0, &[]);

    world.register_system(engine_core::systems::job::JobSystem::new());
    world.register_system(engine_core::systems::movement_system::MovementSystem);

    let mut reached_site = false;
    let mut _completed = false;

    for _tick in 0..40 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();

        let agent_pos_val = world.get_component(agent_id, "Position").unwrap().clone();
        let agent_pos: engine_core::ecs::components::position::PositionComponent =
            serde_json::from_value(agent_pos_val).unwrap();

        let job = world.get_component(job_id, "Job").unwrap();

        let _agent = world.get_component(agent_id, "Agent").unwrap();

        if agent_pos.pos
            == (engine_core::ecs::components::position::Position::Square { x: 2, y: 2, z: 0 })
        {
            reached_site = true;
            // At the site, state should be "at_site" or "in_progress" or "complete"
            let state = job.get("state").unwrap().as_str().unwrap();
            assert!(
                state == "at_site" || state == "in_progress" || state == "complete",
                "Job state should be at_site/in_progress/complete after agent arrives, got {state}"
            );
        }
        if job.get("state") == Some(&serde_json::json!("complete")) {
            _completed = true;
            break;
        }
    }
    assert!(reached_site, "Agent should reach the job site");
    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job.get("state").unwrap(), "complete");
}

#[test]
fn test_job_blocked_when_path_unreachable() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    // Set up a 2x2 grid map with no path between (0,0,0) and (1,1,0)
    use engine_core::map::{Map, SquareGridMap};
    let mut sq_map = SquareGridMap::new();
    sq_map.add_cell(0, 0, 0);
    sq_map.add_cell(1, 1, 0);
    // No neighbor between (0,0,0) and (1,1,0)
    world.map = Some(Map::new(Box::new(sq_map)));

    let agent_id = world.spawn_entity();
    let job_id = world.spawn_entity();

    // Place agent at (0,0,0)
    world
        .set_component(
            agent_id,
            "Position",
            serde_json::to_value(engine_core::ecs::components::position::PositionComponent {
                pos: engine_core::ecs::components::position::Position::Square { x: 0, y: 0, z: 0 },
            })
            .unwrap(),
        )
        .unwrap();

    world
        .set_component(
            agent_id,
            "Agent",
            serde_json::json!({
                "entity_id": agent_id,
                "state": "idle",
                "specializations": [],
                "job_queue": [],
                "move_path": [],
                "carried_resources": []
            }),
        )
        .unwrap();

    // Create a job at unreachable (1,1,0)
    world
        .set_component(
            job_id,
            "Job",
            serde_json::json!({
                "id": job_id,
                "job_type": "dig",
                "state": "pending",
                "priority": 1,
                "category": "mining",
                "target_position": {
                    "pos": {
                        "Square": { "x": 1, "y": 1, "z": 0 }
                    }
                },
                "resource_requirements": [
                    { "kind": "dirt", "amount": 0 }
                ]
            }),
        )
        .unwrap();

    world.job_types.register_job_type(JobTypeData {
        name: "dig".to_string(),
        requirements: vec![],
        duration: None,
        effects: vec![serde_json::json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    });

    let mut job_board = engine_core::systems::job::job_board::JobBoard::default();
    job_board.update(&world, 0, &[]);
    engine_core::systems::job::assign_jobs(&mut world, &mut job_board, 0, &[]);

    world.register_system(engine_core::systems::job::JobSystem::new());

    // Run the job system for a few ticks
    for _ in 0..3 {
        world.run_system("JobSystem").unwrap();
    }

    let job = world.get_component(job_id, "Job").unwrap();
    assert_eq!(job.get("state").unwrap(), "blocked");

    // Agent should be unassigned and idle
    let agent = world.get_component(agent_id, "Agent").unwrap();
    assert!(
        agent.get("current_job").is_none()
            || agent.get("current_job") == Some(&serde_json::Value::Null),
        "Agent should have no current_job (None or null), got: {:?}",
        agent.get("current_job")
    );
    assert_eq!(agent.get("state").unwrap(), "idle");

    // Event should be emitted
    world.update_event_buses::<serde_json::Value>();
    let bus = world
        .get_event_bus::<serde_json::Value>("job_blocked")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert!(!events.is_empty(), "No job_blocked events emitted");
    let found = events.iter().any(|event: &serde_json::Value| {
        event.get("entity").and_then(|v| v.as_u64()) == Some(job_id as u64)
    });
    assert!(found, "No job_blocked event for our job");
}

// --- Section: Dependencies ---

/// Tests that a job with an unfinished dependency remains pending.
#[test]
fn test_job_with_unfinished_dependency_remains_pending() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "id": dep_eid,
                "job_type": "dig",
                "state": "pending",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "id": main_eid,
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    // DO NOT SPAWN AGENT YET!

    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    // Run for MAX_TICKS with no agent, nothing should progress
    for tick in 0..MAX_TICKS {
        job_board.update(&world, tick as u64, &[]);
        engine_core::systems::job::ai::logic::assign_jobs(
            &mut world,
            &mut job_board,
            tick as u64,
            &[],
        );
        job_system.run(&mut world);
    }
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(main_job_after.get("state").unwrap(), "pending");

    // Now spawn agent and manually complete the dependency
    world.spawn_idle_agent();
    let mut dep_job = world.get_component(dep_eid, "Job").unwrap().clone();
    dep_job["state"] = json!("complete");
    dep_job["id"] = json!(dep_eid);
    world.set_component(dep_eid, "Job", dep_job).unwrap();

    // Now main job should progress
    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main_eid, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job_after2 = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after2.get("state").unwrap(), "pending");
}

/// Tests that a job with a completed dependency can start.
#[test]
fn test_job_with_completed_dependency_can_start() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "id": dep_eid,
                "job_type": "dig",
                "state": "complete",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "id": main_eid,
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main_eid, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_ne!(main_job_after.get("state").unwrap(), "pending");
}

/// Tests that a job with AND dependencies can start when all are complete.
#[test]
fn test_job_with_and_dependencies() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let dep2 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"complete","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            dep2,
            "Job",
            json!({"id": dep2, "state":"complete","job_type":"b","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "all_of": [dep1.to_string(), dep2.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with OR dependencies can start when any is complete.
#[test]
fn test_job_with_or_dependencies() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let dep2 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"failed","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            dep2,
            "Job",
            json!({"id": dep2, "state":"complete","job_type":"b","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "any_of": [dep1.to_string(), dep2.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with a NOT dependency does not start if the dependency is failed,
/// and does start after the dependency is despawned.
#[test]
fn test_job_with_not_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep1 = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(
            dep1,
            "Job",
            json!({"id": dep1, "state":"failed","job_type":"a","category":"test"}),
        )
        .unwrap();
    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": { "not": [dep1.to_string()] }
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    // Now remove dep1 (simulate dep1 never existed)
    world.despawn_entity(dep1);

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with a world state dependency remains pending until the resource is available.
#[test]
fn test_job_with_world_state_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();

    let stockpile = world.spawn_entity();
    world
        .set_component(stockpile, "Stockpile", json!({"resources": {}}))
        .unwrap();

    let main = world.spawn_entity();

    world.set_global_resource("water", 5.0);

    world
        .set_component(
            main,
            "Job",
            json!({
                "id": main,
                "job_type":"main",
                "state":"pending",
                "category":"test",
                "dependencies": [
                    { "world_state": { "resource": "water", "gte": 10.0 } }
                ]
            }),
        )
        .unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    world.set_global_resource("water", 10.0);

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

/// Tests that a job with an entity state dependency remains pending until the condition is met.
#[test]
fn test_job_with_entity_state_dependency() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let entity = world.spawn_entity();
    let main = world.spawn_entity();

    world
        .set_component(entity, "Health", json!({"current": 5.0, "max": 10.0}))
        .unwrap();

    world.set_component(main, "Job", json!({
        "id": main,
        "job_type":"main",
        "state":"pending",
        "category":"test",
        "dependencies": [
            { "entity_state": { "entity": entity, "component": "Health", "field": "current", "gte": 10 } }
        ]
    })).unwrap();

    world.spawn_idle_agent();
    let mut job_board = JobBoard::default();
    let mut job_system = JobSystem::new();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_eq!(main_job.get("state").unwrap(), "pending");

    world
        .set_component(entity, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();

    run_until(
        &mut world,
        &mut job_board,
        &mut job_system,
        |world| {
            let main_job = world.get_component(main, "Job").unwrap();
            main_job.get("state").unwrap() != "pending"
        },
        MAX_TICKS,
    );
    let main_job = world.get_component(main, "Job").unwrap();
    assert_ne!(main_job.get("state").unwrap(), "pending");
}

// --- Section: Dependency Failures ---

#[test]
fn test_job_with_failed_dependency_fails() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "state": "failed",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after.get("state").unwrap(),
        "failed",
        "Main job should fail when dependency fails"
    );
}

#[test]
fn test_job_with_cancelled_dependency_cancels() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "state": "cancelled",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction"
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after.get("state").unwrap(),
        "cancelled",
        "Main job should cancel when dependency is cancelled"
    );
}

#[test]
fn test_job_spawns_child_on_dependency_failure() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = world_helper::make_test_world();
    let dep_eid = world.spawn_entity();
    let main_eid = world.spawn_entity();

    world
        .set_component(
            dep_eid,
            "Job",
            json!({
                "job_type": "dig",
                "state": "failed",
                "category": "mining"
            }),
        )
        .unwrap();

    world
        .set_component(
            main_eid,
            "Job",
            json!({
                "job_type": "build",
                "state": "pending",
                "dependencies": [dep_eid.to_string()],
                "category": "construction",
                "on_dependency_failed_spawn": [{
                    "job_type": "notify",
                    "state": "pending",
                    "category": "notification"
                }]
            }),
        )
        .unwrap();

    let mut job_system = JobSystem::new();
    job_system.run(&mut world);

    let main_job_after = world.get_component(main_eid, "Job").unwrap();
    assert_eq!(
        main_job_after.get("state").unwrap(),
        "failed",
        "Main job should fail when dependency fails"
    );
    let children = main_job_after
        .get("children")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(children.len(), 1, "Main job should have one child job");
    assert_eq!(
        children[0].get("job_type").unwrap(),
        "notify",
        "Child job should be of type 'notify'"
    );
    assert_eq!(
        children[0].get("state").unwrap(),
        "pending",
        "Child job should be pending"
    );
    assert_eq!(
        children[0].get("category").unwrap(),
        "notification",
        "Child job should be in the 'notification' category"
    );
}
