use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::faction::{get_faction, get_reputation, modify_reputation, set_faction};
use engine_core::systems::faction_reputation::FactionReputationSystem;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Faction".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/faction.json"))
                .unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
        reg.register_external_schema(ComponentSchema {
            name: "Reputation".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/reputation.json"))
                .unwrap(),
            modes: vec![
                "colony".to_string(),
                "roguelike".to_string(),
                "simulation".to_string(),
            ],
        });
    }
    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();
    world
}

#[test]
fn integration_set_get_faction() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    set_faction(&mut world, e, "goblins", "member").unwrap();
    assert_eq!(get_faction(&world, e), Some("goblins".to_string()));
}

#[test]
fn integration_get_faction_none() {
    let world = setup_world();
    // World::new starts with next_id: 1, entity won't be found
    assert_eq!(get_faction(&world, 999), None);
}

#[test]
fn integration_modify_get_reputation() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    modify_reputation(&mut world, e, "goblins", 25).unwrap();
    assert_eq!(get_reputation(&world, e, "goblins"), 25);
}

#[test]
fn integration_reputation_clamp() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    modify_reputation(&mut world, e, "goblins", 200).unwrap();
    assert_eq!(get_reputation(&world, e, "goblins"), 100);
    modify_reputation(&mut world, e, "goblins", -300).unwrap();
    assert_eq!(get_reputation(&world, e, "goblins"), -100);
}

#[test]
fn integration_reputation_none() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    assert_eq!(get_reputation(&world, e, "goblins"), 0);
}

#[test]
fn integration_reputation_unknown_faction() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    modify_reputation(&mut world, e, "goblins", 25).unwrap();
    assert_eq!(get_reputation(&world, e, "unknown"), 0);
}

#[test]
fn integration_decay_applied() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    world
        .set_component(
            e,
            "Reputation",
            json!({
                "values": { "goblins": 50 },
                "decay_rate": 10.0,
            }),
        )
        .unwrap();
    world.register_system(FactionReputationSystem);
    world.run_system("FactionReputationSystem").unwrap();
    assert_eq!(get_reputation(&world, e, "goblins"), 40);
}

#[test]
fn integration_decay_skipped_zero_rate() {
    let mut world = setup_world();
    let e = world.spawn_entity();
    world
        .set_component(
            e,
            "Reputation",
            json!({
                "values": { "goblins": 50 },
                "decay_rate": 0.0,
            }),
        )
        .unwrap();
    world.register_system(FactionReputationSystem);
    world.run_system("FactionReputationSystem").unwrap();
    assert_eq!(get_reputation(&world, e, "goblins"), 50);
}

// --- System unit tests (moved from inline tests in faction_reputation.rs) ---

#[test]
fn test_decay_positive_toward_zero() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Reputation",
            json!({
                "values": { "goblins": 50 },
                "decay_rate": 10.0,
            }),
        )
        .unwrap();

    let mut system = FactionReputationSystem;
    system.run(&mut world);

    let comp = world.get_component(entity, "Reputation").unwrap();
    let value = comp["values"]["goblins"].as_i64().unwrap();
    assert_eq!(value, 40);
}

#[test]
fn test_decay_negative_toward_zero() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Reputation",
            json!({
                "values": { "goblins": -50 },
                "decay_rate": 10.0,
            }),
        )
        .unwrap();

    let mut system = FactionReputationSystem;
    system.run(&mut world);

    let comp = world.get_component(entity, "Reputation").unwrap();
    let value = comp["values"]["goblins"].as_i64().unwrap();
    assert_eq!(value, -40);
}

#[test]
fn test_decay_skips_zero_decay_rate() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Reputation",
            json!({
                "values": { "goblins": 50 },
                "decay_rate": 0.0,
            }),
        )
        .unwrap();

    let mut system = FactionReputationSystem;
    system.run(&mut world);

    let comp = world.get_component(entity, "Reputation").unwrap();
    let value = comp["values"]["goblins"].as_i64().unwrap();
    assert_eq!(value, 50);
}

#[test]
fn test_decay_does_not_cross_zero() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Reputation",
            json!({
                "values": { "goblins": 5 },
                "decay_rate": 10.0,
            }),
        )
        .unwrap();

    let mut system = FactionReputationSystem;
    system.run(&mut world);

    let comp = world.get_component(entity, "Reputation").unwrap();
    let value = comp["values"]["goblins"].as_i64().unwrap();
    // Should decay to 0, not cross to negative
    assert_eq!(value, 0);
}

#[test]
fn test_decay_does_not_exceed_bounds() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Reputation",
            json!({
                "values": { "goblins": 100 },
                "decay_rate": 10.0,
            }),
        )
        .unwrap();

    let mut system = FactionReputationSystem;
    system.run(&mut world);

    let comp = world.get_component(entity, "Reputation").unwrap();
    let value = comp["values"]["goblins"].as_i64().unwrap();
    assert_eq!(value, 90);
}

#[test]
fn test_system_name() {
    let system = FactionReputationSystem;
    assert_eq!(system.name(), "FactionReputationSystem");
}

#[test]
fn test_system_dependencies() {
    let system = FactionReputationSystem;
    assert!(system.dependencies().is_empty());
}

#[test]
fn test_modify_reputation_emits_event() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let event_name = "reputation_changed".to_string();

    modify_reputation(&mut world, entity, "goblins", 10).unwrap();

    // Advance the event bus to move current events to last_events
    world.update_event_buses::<JsonValue>();

    let drained: Vec<JsonValue> = world.drain_events(&event_name);
    assert!(!drained.is_empty(), "Should have events");
    if let Some(event) = drained.first() {
        assert_eq!(event["entity"], entity);
        assert_eq!(event["faction"], "goblins");
        assert_eq!(event["old"], 0);
        assert_eq!(event["new"], 10);
        assert_eq!(event["delta"], 10);
    }
}
