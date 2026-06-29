use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::faction::{get_faction, get_reputation, modify_reputation, set_faction};
use serde_json::Value as JsonValue;
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
fn test_set_faction_creates_component() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = set_faction(&mut world, entity, "goblins", "member");
    assert!(result.is_ok());

    let comp = world.get_component(entity, "Faction").unwrap();
    assert_eq!(comp["faction_id"], "goblins");
    assert_eq!(comp["role"], "member");
    assert_eq!(comp["joined_tick"], 0);
}

#[test]
fn test_get_faction_returns_faction_id() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    set_faction(&mut world, entity, "humans", "leader").unwrap();
    let result = get_faction(&world, entity);
    assert_eq!(result, Some("humans".to_string()));
}

#[test]
fn test_get_faction_returns_none_when_absent() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = get_faction(&world, entity);
    assert_eq!(result, None);
}

#[test]
fn test_modify_reputation_creates_component() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = modify_reputation(&mut world, entity, "goblins", 10);
    assert!(result.is_ok());

    let comp = world.get_component(entity, "Reputation").unwrap();
    assert_eq!(comp["values"]["goblins"], 10);
    assert_eq!(comp["decay_rate"], 0.0);
}

#[test]
fn test_modify_reputation_clamps_to_max() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    modify_reputation(&mut world, entity, "goblins", 150).unwrap();
    let value = get_reputation(&world, entity, "goblins");
    assert_eq!(value, 100);
}

#[test]
fn test_modify_reputation_clamps_to_min() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    modify_reputation(&mut world, entity, "goblins", -150).unwrap();
    let value = get_reputation(&world, entity, "goblins");
    assert_eq!(value, -100);
}

#[test]
fn test_modify_reputation_emits_event() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let event_name = "reputation_changed".to_string();

    modify_reputation(&mut world, entity, "goblins", 10).unwrap();

    // Advance the event bus to move current events to last_events
    world.update_event_buses::<JsonValue>();

    // Use drain_events to check
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

#[test]
fn test_get_reputation_returns_zero_when_absent() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    let result = get_reputation(&world, entity, "goblins");
    assert_eq!(result, 0);
}

#[test]
fn test_get_reputation_returns_zero_for_unknown_faction() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    modify_reputation(&mut world, entity, "goblins", 25).unwrap();
    let result = get_reputation(&world, entity, "humans");
    assert_eq!(result, 0);
}

#[test]
fn test_get_reputation_returns_correct_value() {
    let mut world = setup_world();
    let entity = world.spawn_entity();
    modify_reputation(&mut world, entity, "goblins", 25).unwrap();
    let result = get_reputation(&world, entity, "goblins");
    assert_eq!(result, 25);
}
