use crate::ecs::world::World;
use serde_json::{json, Value as JsonValue};

pub fn set_faction(
    world: &mut World,
    entity: u32,
    faction_id: &str,
    role: &str,
) -> Result<(), String> {
    let joined_tick = world.turn;
    world.set_component(
        entity,
        "Faction",
        json!({
            "faction_id": faction_id,
            "role": role,
            "joined_tick": joined_tick,
        }),
    )
}

pub fn get_faction(world: &World, entity: u32) -> Option<String> {
    let component = world.get_component(entity, "Faction")?;
    let faction_id = component.get("faction_id")?.as_str()?;
    Some(faction_id.to_string())
}

pub fn modify_reputation(
    world: &mut World,
    target_entity: u32,
    faction_id: &str,
    delta: i64,
) -> Result<(), String> {
    // Read current reputation component or create a default one
    let old_value = world
        .get_component(target_entity, "Reputation")
        .and_then(|c| c.get("values"))
        .and_then(|v| v.get(faction_id))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let new_value = (old_value + delta).clamp(-100, 100);

    // Build the values map
    let mut values_map = world
        .get_component(target_entity, "Reputation")
        .and_then(|c| c.get("values"))
        .and_then(|v| v.as_object())
        .map(|m| {
            let mut map = serde_json::Map::new();
            for (k, v) in m {
                map.insert(k.clone(), v.clone());
            }
            map
        })
        .unwrap_or_default();

    values_map.insert(faction_id.to_string(), json!(new_value));

    world.set_component(
        target_entity,
        "Reputation",
        json!({
            "values": JsonValue::Object(values_map),
            "decay_rate": world
                .get_component(target_entity, "Reputation")
                .and_then(|c| c.get("decay_rate"))
                .cloned()
                .unwrap_or(json!(0.0)),
        }),
    )?;

    // Emit event
    world.send_event(
        "reputation_changed",
        json!({
            "entity": target_entity,
            "faction": faction_id,
            "old": old_value,
            "new": new_value,
            "delta": delta,
        }),
    )?;

    Ok(())
}

pub fn get_reputation(world: &World, target_entity: u32, faction_id: &str) -> i64 {
    world
        .get_component(target_entity, "Reputation")
        .and_then(|c| c.get("values"))
        .and_then(|v| v.get(faction_id))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::registry::ComponentRegistry;
    use crate::ecs::schema::ComponentSchema;
    use std::sync::{Arc, Mutex};

    fn setup_world() -> World {
        let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
        // Register schemas so mode enforcement passes
        {
            let mut reg = registry.lock().unwrap();
            let _ = reg.register_external_schema(ComponentSchema {
                name: "Faction".to_string(),
                schema: serde_json::from_str(include_str!(
                    "../../assets/schemas/faction.json"
                ))
                .unwrap(),
                modes: vec!["colony".to_string(), "roguelike".to_string(), "simulation".to_string()],
            });
            let _ = reg.register_external_schema(ComponentSchema {
                name: "Reputation".to_string(),
                schema: serde_json::from_str(include_str!(
                    "../../assets/schemas/reputation.json"
                ))
                .unwrap(),
                modes: vec!["colony".to_string(), "roguelike".to_string(), "simulation".to_string()],
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
}
