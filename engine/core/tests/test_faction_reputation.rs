use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::faction::{get_faction, get_reputation, modify_reputation, set_faction};
use engine_core::systems::faction_reputation::FactionReputationSystem;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn setup_world() -> World {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        let _ = reg.register_external_schema(ComponentSchema {
            name: "Faction".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/faction.json"))
                .unwrap(),
            modes: vec!["colony".to_string(), "roguelike".to_string()],
        });
        let _ = reg.register_external_schema(ComponentSchema {
            name: "Reputation".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/reputation.json"))
                .unwrap(),
            modes: vec!["colony".to_string(), "roguelike".to_string()],
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
