#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::system::System;
use engine_core::systems::job::job_board::JobBoard;
use engine_core::systems::job::{JobSystem, assign_jobs};
use serde_json::json;

#[test]
fn test_job_progression_over_ticks() {
    let mut world = world_helper::make_test_world();

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            100,
            "Job",
            json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();
    world.entities.push(100);

    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let mut job_system = JobSystem::new();
    for _ in 0..5 {
        job_system.run(&mut world, None);
        let job = world.get_component(100, "Job").unwrap();
        let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let status = job.get("status").and_then(|v| v.as_str()).unwrap();
        if progress < 3.0 {
            assert_eq!(
                status, "in_progress",
                "Job should be in progress while progress < 3.0"
            );
        } else {
            assert_eq!(
                status, "complete",
                "Job should be complete when progress >= 3.0"
            );
            break;
        }
    }
    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(
        job.get("status").unwrap(),
        "complete",
        "Job should be complete after progression"
    );
}

#[test]
fn test_custom_job_handler_overrides_progression() {
    let mut world = world_helper::make_test_world();

    {
        let registry = world.job_handler_registry.clone();
        registry.lock().unwrap().register_handler(
            "instant",
            |_world, _agent_id, _job_id, job: &serde_json::Value| {
                let mut job = job.clone();
                job["progress"] = serde_json::json!(42.0);
                job["status"] = serde_json::json!("complete");
                job
            },
        );
    }

    world
        .set_component(
            1,
            "Agent",
            json!({
                "entity_id": 1,
                "state": "idle"
            }),
        )
        .unwrap();
    world.entities.push(1);

    world
        .set_component(
            101,
            "Job",
            json!({
                "id": 101,
                "job_type": "instant",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "testing"
            }),
        )
        .unwrap();
    world.entities.push(101);

    let mut job_board = JobBoard::default();
    job_board.update(&world);
    assign_jobs(&mut world, &mut job_board);

    let mut job_system = JobSystem::new();
    job_system.run(&mut world, None);

    let job = world.get_component(101, "Job").unwrap();
    assert_eq!(
        job.get("progress").unwrap(),
        42.0,
        "Progress should be set by custom handler"
    );
    assert_eq!(
        job.get("status").unwrap(),
        "complete",
        "Status should be set by custom handler"
    );
}

#[test]
fn test_effects_applied_only_on_completion_and_rolled_back_on_cancel() {
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

    world
        .set_component(200, "Terrain", json!({ "kind": "rock" }))
        .unwrap();

    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "cancelled": false,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    world.job_types.register_job_type(
        "dig",
        vec![json!({
            "action": "ModifyTerrain",
            "from": "rock",
            "to": "tunnel"
        })],
    );

    {
        let mut job_board = JobBoard::default();
        job_board.update(&world);
        assign_jobs(&mut world, &mut job_board);

        let mut job_system = JobSystem::new();
        for _ in 0..5 {
            job_system.run(&mut world, None);
        }

        let terrain = world.get_component(200, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "tunnel",
            "Terrain should change to tunnel after job completion"
        );
    }

    world
        .set_component(200, "Terrain", json!({ "kind": "rock" }))
        .unwrap();
    world
        .set_component(
            200,
            "Job",
            json!({
                "id": 200,
                "job_type": "dig",
                "status": "pending",
                "cancelled": true,
                "priority": 1,
                "category": "mining"
            }),
        )
        .unwrap();

    {
        let mut job_system = JobSystem::new();
        job_system.run(&mut world, None);

        let terrain = world.get_component(200, "Terrain").unwrap();
        assert_eq!(
            terrain["kind"], "rock",
            "Terrain should remain rock after job cancellation"
        );
    }
}

#[test]
fn test_agent_moves_to_job_site_before_progress() {
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

    // --- Register agent and job BEFORE assignment ---
    world
        .set_component(
            1,
            "Position",
            serde_json::to_value(engine_core::ecs::components::position::PositionComponent {
                pos: engine_core::ecs::components::position::Position::Square { x: 0, y: 0, z: 0 },
            })
            .unwrap(),
        )
        .unwrap();

    world
        .set_component(
            1,
            "Agent",
            serde_json::json!({
                "entity_id": 1,
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
            100,
            "Job",
            serde_json::json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "phase": "pending",
                "cancelled": false,
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

    world.entities.push(1);
    world.entities.push(100);

    let mut job_board = engine_core::systems::job::job_board::JobBoard::default();
    job_board.update(&world);
    engine_core::systems::job::assign_jobs(&mut world, &mut job_board);

    world.register_system(engine_core::systems::job::JobSystem::new());
    world.register_system(engine_core::systems::movement_system::MovementSystem);

    let mut reached_site = false;

    for _tick in 0..20 {
        world.run_system("MovementSystem", None).unwrap();
        world.run_system("JobSystem", None).unwrap();

        let agent_pos_val = world.get_component(1, "Position").unwrap().clone();
        let agent_pos: engine_core::ecs::components::position::PositionComponent =
            serde_json::from_value(agent_pos_val).unwrap();

        let job = world.get_component(100, "Job").unwrap();

        if agent_pos.pos
            == (engine_core::ecs::components::position::Position::Square { x: 2, y: 2, z: 0 })
        {
            reached_site = true;
            assert_eq!(job.get("phase").unwrap(), "at_site");
            break;
        }
    }
    assert!(reached_site, "Agent should reach the job site");
    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(job.get("status").unwrap(), "complete");
}

#[test]
fn test_job_blocked_when_path_unreachable() {
    let mut world = world_helper::make_test_world();

    // Set up a 2x2 grid map with no path between (0,0,0) and (1,1,0)
    use engine_core::map::{Map, SquareGridMap};
    let mut sq_map = SquareGridMap::new();
    sq_map.add_cell(0, 0, 0);
    sq_map.add_cell(1, 1, 0);
    // No neighbor between (0,0,0) and (1,1,0)
    world.map = Some(Map::new(Box::new(sq_map)));

    // Place agent at (0,0,0)
    world
        .set_component(
            1,
            "Position",
            serde_json::to_value(engine_core::ecs::components::position::PositionComponent {
                pos: engine_core::ecs::components::position::Position::Square { x: 0, y: 0, z: 0 },
            })
            .unwrap(),
        )
        .unwrap();

    world
        .set_component(
            1,
            "Agent",
            serde_json::json!({
                "entity_id": 1,
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
            100,
            "Job",
            serde_json::json!({
                "id": 100,
                "job_type": "dig",
                "status": "pending",
                "phase": "pending",
                "cancelled": false,
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

    world.entities.push(1);
    world.entities.push(100);

    let mut job_board = engine_core::systems::job::job_board::JobBoard::default();
    job_board.update(&world);
    engine_core::systems::job::assign_jobs(&mut world, &mut job_board);

    world.register_system(engine_core::systems::job::JobSystem::new());

    // Run the job system for a few ticks
    for _ in 0..3 {
        world.run_system("JobSystem", None).unwrap();
    }

    let job = world.get_component(100, "Job").unwrap();
    assert_eq!(job.get("phase").unwrap(), "blocked");
    assert_eq!(job.get("status").unwrap(), "blocked");

    // Agent should be unassigned and idle
    let agent = world.get_component(1, "Agent").unwrap();
    assert!(agent.get("current_job").is_none());
    assert_eq!(agent.get("state").unwrap(), "idle");

    // Event should be emitted
    world.update_event_buses::<serde_json::Value>();
    let bus = world
        .get_event_bus::<serde_json::Value>("job_blocked")
        .expect("event bus exists");
    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert!(!events.is_empty(), "No job_blocked events emitted");
    let found = events
        .iter()
        .any(|event: &serde_json::Value| event.get("entity").and_then(|v| v.as_u64()) == Some(100));
    assert!(found, "No job_blocked event for our job");
}
