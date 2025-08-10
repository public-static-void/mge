use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// System for movement
#[derive(Default)]
pub struct MovementSystem;

impl System for MovementSystem {
    fn name(&self) -> &'static str {
        "MovementSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let agent_eids = world.get_entities_with_component("Agent");
        for eid in agent_eids {
            let mut agent = match world.get_component(eid, "Agent").cloned() {
                Some(agent) => agent,
                None => continue,
            };

            // Only process agents with a move_path
            let move_path = match agent.get_mut("move_path") {
                Some(JsonValue::Array(path)) if !path.is_empty() => path,
                _ => continue,
            };

            // Next step in path as JSON
            let next_pos_val = move_path.remove(0);

            // Always set as { "pos": { "Square": ... } } (not double-wrapped!)
            // If next_pos_val is { "pos": { ... } }, use as-is.
            // If next_pos_val is { "Square": ... }, wrap as { "pos": ... }
            let new_position = if next_pos_val.get("pos").is_some() {
                next_pos_val
            } else if next_pos_val.get("Square").is_some()
                || next_pos_val.get("Hex").is_some()
                || next_pos_val.get("Region").is_some()
            {
                serde_json::json!({ "pos": next_pos_val })
            } else {
                // Defensive fallback: do not set an invalid position
                continue;
            };

            let _ = world.set_component(eid, "Position", new_position);

            // If move_path is now empty, remove it
            if move_path.is_empty() {
                agent.as_object_mut().unwrap().remove("move_path");
            }
            let _ = world.set_component(eid, "Agent", agent);
        }
    }
}
