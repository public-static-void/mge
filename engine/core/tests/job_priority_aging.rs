use engine_core::config::GameConfig;
use engine_core::ecs::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::systems::job::priority_aging::JobPriorityAgingSystem;
use engine_core::systems::job_board::{JobAssignmentResult, JobBoard};
use serde_json::json;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
    let config =
        GameConfig::load_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"))
            .expect("Failed to load config");
    let schema_dir = "../../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    let registry = Arc::new(Mutex::new(registry));
    World::new(registry)
}

#[test]
fn high_priority_job_is_assigned_first() {
    let mut world = setup_world();

    let high_eid = world.spawn_entity();
    let low_eid = world.spawn_entity();

    world
        .set_component(
            high_eid,
            "Job",
            json!({
                "id": high_eid,
                "job_type": "urgent",
                "status": "pending",
                "priority": 100,
                "creation_tick": 0,
                "category": "priority"
            }),
        )
        .unwrap();

    world
        .set_component(
            low_eid,
            "Job",
            json!({
                "id": low_eid,
                "job_type": "background",
                "status": "pending",
                "priority": 1,
                "creation_tick": 0,
                "category": "background"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Update priorities for tick 0
    let mut aging_system = JobPriorityAgingSystem::new();
    aging_system.run(&mut world, 0);

    let mut job_board = JobBoard::default();
    job_board.update(&world);
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::Assigned(high_eid));
}

#[test]
fn low_priority_job_is_assigned_after_aging() {
    let mut world = setup_world();

    let high_eid = world.spawn_entity();
    let low_eid = world.spawn_entity();

    world
        .set_component(
            high_eid,
            "Job",
            json!({
                "id": high_eid,
                "job_type": "urgent",
                "status": "pending",
                "priority": 100,
                "creation_tick": 0,
                "category": "priority"
            }),
        )
        .unwrap();

    world
        .set_component(
            low_eid,
            "Job",
            json!({
                "id": low_eid,
                "job_type": "background",
                "status": "pending",
                "priority": 1,
                "creation_tick": 0,
                "category": "background"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Assign and complete the high-priority job
    let mut aging_system = JobPriorityAgingSystem::new();
    aging_system.run(&mut world, 0);
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    let result = job_board.claim_job(agent_eid, &mut world, 0);
    assert_eq!(result, JobAssignmentResult::Assigned(high_eid));
    let mut job = world.get_component(high_eid, "Job").unwrap().clone();
    job["status"] = json!("complete");
    world.set_component(high_eid, "Job", job).unwrap();
    let mut agent = world.get_component(agent_eid, "Agent").unwrap().clone();
    agent["state"] = json!("idle");
    world.set_component(agent_eid, "Agent", agent).unwrap();

    // Simulate many ticks to age the low-priority job
    let mut assigned = false;
    for tick in 1..=200 {
        aging_system.run(&mut world, tick);
        job_board.update(&world);
        let result = job_board.claim_job(agent_eid, &mut world, tick);
        if result == JobAssignmentResult::Assigned(low_eid) {
            assigned = true;
            break;
        }
        // Reset agent to idle for next tick
        let mut agent = world.get_component(agent_eid, "Agent").unwrap().clone();
        agent["state"] = json!("idle");
        world.set_component(agent_eid, "Agent", agent).unwrap();
    }
    assert!(assigned, "Low-priority job was not assigned after aging");
}

#[test]
fn job_priority_can_be_bumped_by_world_event() {
    let mut world = setup_world();

    let job_eid = world.spawn_entity();
    world
        .set_component(
            job_eid,
            "Job",
            json!({
                "id": job_eid,
                "job_type": "critical",
                "status": "pending",
                "priority": 10,
                "creation_tick": 0,
                "category": "critical"
            }),
        )
        .unwrap();

    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Simulate a world event that bumps the job's priority
    let mut job = world.get_component(job_eid, "Job").unwrap().clone();
    job["priority"] = json!(1000);
    world.set_component(job_eid, "Job", job).unwrap();

    // Recompute effective priorities
    let mut aging_system = JobPriorityAgingSystem::new();
    aging_system.run(&mut world, 1);

    let mut job_board = JobBoard::default();
    job_board.update(&world);
    let result = job_board.claim_job(agent_eid, &mut world, 1);
    assert_eq!(result, JobAssignmentResult::Assigned(job_eid));
}

#[test]
fn jobs_get_priority_boost_on_resource_shortage_event() {
    use engine_core::systems::job::priority_aging::JobPriorityAgingSystem;
    use engine_core::systems::job_board::{JobAssignmentResult, JobBoard};
    use serde_json::json;

    let mut world = setup_world();

    // Add a stockpile with enough wood and stone
    let stockpile_eid = world.spawn_entity();
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            json!({ "resources": { "wood": 10, "stone": 10 } }),
        )
        .unwrap();

    // Agent
    let agent_eid = world.spawn_entity();
    world
        .set_component(
            agent_eid,
            "Agent",
            json!({ "entity_id": agent_eid, "state": "idle" }),
        )
        .unwrap();

    // Two jobs: one needs "wood", one needs "stone"
    let wood_job_eid = world.spawn_entity();
    world
        .set_component(
            wood_job_eid,
            "Job",
            json!({
                "id": wood_job_eid,
                "job_type": "build",
                "status": "pending",
                "priority": 1,
                "resource_requirements": [{ "kind": "wood", "amount": 5 }],
                "creation_tick": 0,
                "category": "construction"
            }),
        )
        .unwrap();

    let stone_job_eid = world.spawn_entity();
    world
        .set_component(
            stone_job_eid,
            "Job",
            json!({
                "id": stone_job_eid,
                "job_type": "build",
                "status": "pending",
                "priority": 1,
                "resource_requirements": [{ "kind": "stone", "amount": 5 }],
                "creation_tick": 0,
                "category": "construction"
            }),
        )
        .unwrap();

    // Run resource reservation system so jobs can be reserved
    let mut reservation_system =
        engine_core::systems::job::resource_reservation::ResourceReservationSystem::new();
    reservation_system.run(&mut world, None);

    // Emit a resource shortage event for "wood"
    world
        .send_event("resource_shortage", json!({ "kind": "wood" }))
        .unwrap();

    // DELIVER THE EVENT so it will be seen by the aging system
    world.update_event_buses::<serde_json::Value>();

    // Run priority aging system to process the event and apply boost
    let mut aging_system = JobPriorityAgingSystem::new();
    aging_system.run(&mut world, 1);

    // Update job board and assign jobs
    let mut job_board = JobBoard::default();
    job_board.update(&world);
    let result = job_board.claim_job(agent_eid, &mut world, 1);

    // The wood job should get a priority boost and be assigned
    assert_eq!(result, JobAssignmentResult::Assigned(wood_job_eid));

    let wood_job_effective = world
        .get_component(wood_job_eid, "Job")
        .and_then(|j| j.get("effective_priority").and_then(|v| v.as_i64()))
        .unwrap_or(0);

    let stone_job_effective = world
        .get_component(stone_job_eid, "Job")
        .and_then(|j| j.get("effective_priority").and_then(|v| v.as_i64()))
        .unwrap_or(0);

    assert!(
        wood_job_effective > stone_job_effective,
        "Wood job should have received a dynamic priority boost"
    );
}
