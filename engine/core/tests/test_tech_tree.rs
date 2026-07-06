use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::research::ResearchSystem;
use engine_core::tech_tree::{
    self, cancel_research, can_research_tech, clear_research_queue, get_completed_techs,
    get_research_queue, get_research_queue_progress, get_tech_node, get_tech_progress,
    is_tech_completed, research_tech,
};
use serde_json::{json, Value as JsonValue};
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "TechProgress".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/tech_progress.json"))
                .unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
        reg.register_external_schema(ComponentSchema {
            name: "SkillLevels".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/skill_levels.json"))
                .unwrap(),
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
    }
    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();
    world
}

#[test]
fn test_get_tech_tree_returns_nodes() {
    let _world = setup_world();
    let tree = tech_tree::get_tech_tree();
    assert!(!tree.is_empty(), "Tech tree should have nodes");
    let ids: Vec<&str> = tree.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"bronze_working"), "Should contain bronze_working");
    assert!(ids.contains(&"iron_working"), "Should contain iron_working");
}

#[test]
fn test_get_tech_node_exists() {
    let _world = setup_world();
    let node = get_tech_node("bronze_working");
    assert!(node.is_some(), "bronze_working should exist");
    assert_eq!(node.unwrap().name, "Bronze Working");
}

#[test]
fn test_get_tech_node_missing() {
    let _world = setup_world();
    let node = get_tech_node("nonexistent_tech");
    assert!(node.is_none(), "Nonexistent tech should return None");
}

#[test]
fn test_get_tech_progress_returns_none_when_absent() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let progress = get_tech_progress(&world, entity);
    assert!(progress.is_none(), "Entity without TechProgress should return None");
}

#[test]
fn test_get_completed_techs_empty_initially() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {}, "queue": [], "queue_progress": {}, "research_points": 0.0}),
    ).unwrap();
    let completed = get_completed_techs(&world, entity);
    assert!(completed.is_empty(), "New TechProgress should have no completed techs");
}

#[test]
fn test_is_tech_completed_false_initially() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {}, "queue": [], "queue_progress": {}, "research_points": 0.0}),
    ).unwrap();
    assert!(!is_tech_completed(&world, entity, "bronze_working"));
}

#[test]
fn test_research_tech_adds_to_queue() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = research_tech(&mut world, entity, "bronze_working");
    assert!(result.is_ok(), "research_tech should succeed: {:?}", result.err());
    let queue = get_research_queue(&world, entity);
    assert_eq!(queue.len(), 1, "Queue should have 1 entry");
    assert_eq!(queue[0], "bronze_working");
}

#[test]
fn test_research_tech_fails_when_already_completed() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {"bronze_working": 1}, "queue": [], "queue_progress": {}, "research_points": 0.0}),
    ).unwrap();
    let result = research_tech(&mut world, entity, "bronze_working");
    assert!(result.is_err(), "Should fail when tech already completed");
    assert!(result.unwrap_err().contains("already completed"), "Error should mention already completed");
}

#[test]
fn test_research_tech_fails_with_unmet_prereq() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = research_tech(&mut world, entity, "iron_working");
    assert!(result.is_err(), "Should fail when prerequisite not met");
    let err = result.unwrap_err();
    assert!(err.contains("bronze_working"), "Error should mention missing prereq: {err}");
}

#[test]
fn test_research_tech_fails_with_skill_prereq_not_met() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    // Complete bronze_working and iron_working
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {"bronze_working": 1, "iron_working": 1}, "queue": [], "queue_progress": {}, "research_points": 200.0}),
    ).unwrap();
    // Set skill levels - give metalworking level 1 (needs 5)
    world.set_component(
        entity, "SkillLevels",
        json!({"skills": {}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {"metalworking": 1.0}}),
    ).unwrap();
    let result = research_tech(&mut world, entity, "advanced_metallurgy");
    assert!(result.is_err(), "Should fail when skill prereq not met");
    assert!(result.unwrap_err().contains("metalworking"), "Error should mention missing skill");
}

#[test]
fn test_research_tech_already_in_queue_fails() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    research_tech(&mut world, entity, "bronze_working").unwrap();
    let result = research_tech(&mut world, entity, "bronze_working");
    assert!(result.is_err(), "Should fail when already in queue");
    assert!(result.unwrap_err().contains("already in research queue"), "Error should mention already queued");
}

#[test]
fn test_can_research_tech_true() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = can_research_tech(&world, entity, "bronze_working");
    assert!(result.is_ok(), "can_research_tech should be Ok");
    assert!(result.unwrap(), "bronze_working should be researchable");
}

#[test]
fn test_can_research_tech_false_for_missing_prereq() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = can_research_tech(&world, entity, "iron_working");
    assert!(result.is_err(), "Should error when prereq not met");
    assert!(result.unwrap_err().contains("bronze_working"), "Error should mention missing prereq");
}

#[test]
fn test_can_research_tech_fails_for_unknown_tech() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = can_research_tech(&world, entity, "unknown_tech");
    assert!(result.is_err(), "Should error for unknown tech");
    assert!(result.unwrap_err().contains("Unknown"), "Error should say unknown");
}

#[test]
fn test_cancel_research_removes_from_queue() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    research_tech(&mut world, entity, "bronze_working").unwrap();
    assert_eq!(get_research_queue(&world, entity).len(), 1);
    cancel_research(&mut world, entity, "bronze_working").unwrap();
    assert!(get_research_queue(&world, entity).is_empty(), "Queue should be empty after cancel");
}

#[test]
fn test_cancel_research_fails_for_not_in_queue() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = cancel_research(&mut world, entity, "bronze_working");
    assert!(result.is_err(), "Should fail when tech not in queue");
}

#[test]
fn test_clear_research_queue_empties_queue() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    research_tech(&mut world, entity, "bronze_working").unwrap();
    assert_eq!(get_research_queue(&world, entity).len(), 1);
    clear_research_queue(&mut world, entity).unwrap();
    assert!(get_research_queue(&world, entity).is_empty(), "Queue should be empty after clear");
}

#[test]
fn test_get_research_queue_progress() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    research_tech(&mut world, entity, "bronze_working").unwrap();
    let qp = get_research_queue_progress(&world, entity);
    assert!(qp.as_object().is_some(), "queue_progress should be an object");
    let progress = qp["bronze_working"].as_f64().unwrap_or(-1.0);
    assert_eq!(progress, 0.0, "Initial progress should be 0");
}

#[test]
fn test_research_system_allocates_points() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    research_tech(&mut world, entity, "bronze_working").unwrap();
    let mut system = ResearchSystem;
    system.run(&mut world);
    let progress = get_tech_progress(&world, entity).unwrap();
    let qp = progress["queue_progress"]["bronze_working"].as_f64().unwrap_or(0.0);
    assert!(qp > 0.0, "System should allocate points to queued tech (got {qp})");
}

#[test]
fn test_research_system_completes_tech() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {}, "queue": ["bronze_working"], "queue_progress": {"bronze_working": 49.0}, "research_points": 100.0}),
    ).unwrap();
    assert!(!is_tech_completed(&world, entity, "bronze_working"));
    let mut system = ResearchSystem;
    system.run(&mut world);
    assert!(is_tech_completed(&world, entity, "bronze_working"), "bronze_working should be completed");
    assert!(get_research_queue(&world, entity).is_empty(), "Queue should be empty after tech completion");
}

#[test]
fn test_research_system_emits_event_on_completion() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {}, "queue": ["bronze_working"], "queue_progress": {"bronze_working": 49.0}, "research_points": 100.0}),
    ).unwrap();
    let mut system = ResearchSystem;
    system.run(&mut world);
    world.update_event_buses::<JsonValue>();
    let events = world.drain_events::<JsonValue>("tech_unlocked");
    assert!(!events.is_empty(), "tech_unlocked event should be emitted");
    assert_eq!(events[0]["tech_id"].as_str(), Some("bronze_working"), "Event should contain tech_id");
    assert_eq!(events[0]["entity"].as_u64(), Some(entity as u64), "Event should contain entity");
}

#[test]
fn test_research_tech_unknown_tech_fails() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = research_tech(&mut world, entity, "completely_unknown_tech");
    assert!(result.is_err(), "Should fail for unknown tech");
    assert!(result.unwrap_err().contains("Unknown"), "Error should say Unknown");
}

#[test]
fn test_research_system_handles_empty_queue() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world.set_component(
        entity, "TechProgress",
        json!({"completed": {}, "queue": [], "queue_progress": {}, "research_points": 5.0}),
    ).unwrap();
    let mut system = ResearchSystem;
    system.run(&mut world);
    // Should not crash, points should accumulate
    let progress = get_tech_progress(&world, entity).unwrap();
    assert_eq!(progress["research_points"].as_f64().unwrap_or(0.0), 5.0, "Points should not change when queue is empty");
}

#[test]
fn test_research_system_multiple_entities() {
    let mut world = setup_world();
    let e1 = world.spawn_entity();
    let e2 = world.spawn_entity();
    research_tech(&mut world, e1, "bronze_working").unwrap();
    research_tech(&mut world, e2, "bronze_working").unwrap();
    let mut system = ResearchSystem;
    system.run(&mut world);
    let p1 = get_research_queue_progress(&world, e1);
    let p2 = get_research_queue_progress(&world, e2);
    assert!(p1["bronze_working"].as_f64().unwrap_or(0.0) > 0.0, "e1 should have progress");
    assert!(p2["bronze_working"].as_f64().unwrap_or(0.0) > 0.0, "e2 should have progress");
}
