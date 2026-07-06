//! Research system: allocates research points and completes techs.
//!
//! Each tick, entities with a TechProgress component gain 1 research point.
//! Points are allocated to the front of the research queue. When a tech's
//! cost is met, it is marked as completed and a `tech_unlocked` event fires.

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::tech_tree;
use serde_json::json;

/// System that processes research each tick.
///
/// Registered as "ResearchSystem" in the system execution order.
pub struct ResearchSystem;

impl System for ResearchSystem {
    fn name(&self) -> &'static str {
        "ResearchSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&mut self, world: &mut World) {
        // Step 1: Find all entities with TechProgress component
        let entities = world.get_entities_with_component("TechProgress");

        for entity in entities {
            // Step 2: Read current TechProgress
            let progress = match world.get_component(entity, "TechProgress") {
                Some(v) => v.clone(),
                None => continue,
            };

            let mut completed = progress["completed"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let mut queue: Vec<String> = progress["queue"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let mut queue_progress = progress["queue_progress"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            let mut research_points = progress["research_points"]
                .as_f64()
                .unwrap_or(0.0);

            if queue.is_empty() {
                continue; // Nothing to research
            }

            // Step 3: Add base research points (1.0 per tick)
            research_points += 1.0;

            // Step 4: Allocate to front of queue until points exhausted
            while research_points > 0.0 && !queue.is_empty() {
                let front_id = queue[0].clone();
                let node = match tech_tree::get_tech_node(&front_id) {
                    Some(n) => n,
                    None => {
                        // Unknown tech in queue — remove it
                        queue.remove(0);
                        queue_progress.remove(&front_id);
                        continue;
                    }
                };

                // Allocate points
                let current = queue_progress
                    .get(&front_id)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let allocated = research_points.min(node.cost - current);
                research_points -= allocated;
                queue_progress.insert(front_id.clone(), json!(current + allocated));

                // Check if completed
                let new_current = current + allocated;
                if new_current >= node.cost {
                    // Move to completed
                    completed.insert(front_id.clone(), json!(world.turn));
                    queue.remove(0);
                    queue_progress.remove(&front_id);

                    // Fire tech_unlocked event
                    let _ = world.send_event(
                        "tech_unlocked",
                        json!({
                            "entity": entity,
                            "tech_id": front_id,
                            "tech_name": node.name,
                            "tick": world.turn,
                        }),
                    );
                } else {
                    break; // Can't complete front tech, stop allocating
                }
            }

            // Step 5: Write updated TechProgress
            let updated = json!({
                "completed": serde_json::Value::Object(completed),
                "queue": queue,
                "queue_progress": serde_json::Value::Object(queue_progress),
                "research_points": research_points,
            });
            let _ = world.set_component(entity, "TechProgress", updated);
        }
    }
}
