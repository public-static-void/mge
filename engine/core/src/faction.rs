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
