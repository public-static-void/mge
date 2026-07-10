use crate::World;
use crate::ecs::system::System;
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

use super::recipe::{Recipe, ResourceAmount};

/// Economic system
#[derive(Default)]
pub struct EconomicSystem {
    recipes: HashMap<String, Recipe>,
}

impl EconomicSystem {
    /// Create a new economic system
    pub fn with_recipes(recipes: Vec<Recipe>) -> Self {
        let mut map = HashMap::new();
        for recipe in recipes {
            map.insert(recipe.name.clone(), recipe);
        }
        Self { recipes: map }
    }

    fn can_consume_inputs(
        stockpile: &mut serde_json::Map<String, JsonValue>,
        inputs: &[ResourceAmount],
    ) -> bool {
        for input in inputs {
            let current = stockpile
                .get(&input.kind)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if current < input.amount {
                return false;
            }
        }
        true
    }

    fn consume_inputs(
        stockpile: &mut serde_json::Map<String, JsonValue>,
        inputs: &[ResourceAmount],
    ) {
        for input in inputs {
            let current = stockpile
                .get(&input.kind)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            stockpile.insert(input.kind.clone(), json!(current - input.amount));
        }
    }

    fn produce_outputs(
        stockpile: &mut serde_json::Map<String, JsonValue>,
        outputs: &[ResourceAmount],
    ) {
        for output in outputs {
            let current = stockpile
                .get(&output.kind)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            stockpile.insert(output.kind.clone(), json!(current + output.amount));
        }
    }
}

impl System for EconomicSystem {
    fn name(&self) -> &'static str {
        "EconomicSystem"
    }

    fn run(&mut self, world: &mut World) {
        // Collect (entity_id, priority) pairs and sort by priority descending,
        // tie-breaking by ascending entity ID for deterministic order.
        let mut entity_priorities: Vec<(u32, i64)> = world
            .get_entities_with_component("ProductionJob")
            .into_iter()
            .filter_map(|eid| {
                let job = world.get_component(eid, "ProductionJob")?;
                let priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                Some((eid, priority))
            })
            .collect();
        entity_priorities.sort_by(|a, b| {
            // Higher priority first (descending), then lower entity ID first (ascending)
            b.1.cmp(&a.1).then(a.0.cmp(&b.0))
        });

        for (eid, _priority) in entity_priorities {
            let Some(mut job) = world.get_component(eid, "ProductionJob").cloned() else {
                continue;
            };
            let state = job
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");

            // Skip already completed jobs
            if state == "complete" {
                continue;
            }

            // Transition pending → in_progress when assigned to a worker
            if state == "pending" && job.get("assigned_to").and_then(|v| v.as_u64()).is_some() {
                job["state"] = json!("in_progress");
            }

            let recipe_name = match job.get("recipe").and_then(|v| v.as_str()) {
                Some(r) => r.to_string(),
                None => continue,
            };
            let recipe = match self.recipes.get(&recipe_name) {
                Some(r) => r,
                None => continue,
            };
            let Some(mut stockpile) = world.get_component(eid, "Stockpile").cloned() else {
                continue;
            };
            let Some(stock_map) = stockpile
                .get_mut("resources")
                .and_then(|v| v.as_object_mut())
            else {
                continue;
            };

            if Self::can_consume_inputs(stock_map, &recipe.inputs) {
                Self::consume_inputs(stock_map, &recipe.inputs);

                let progress = job.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
                job["progress"] = json!(progress);

                if progress >= recipe.duration {
                    // Read batch_size (default 1, minimum 1)
                    let batch_size = job
                        .get("batch_size")
                        .and_then(|v| v.as_i64())
                        .filter(|&v| v >= 1)
                        .unwrap_or(1);

                    // Produce outputs multiplied by batch_size
                    let multiplied_outputs: Vec<super::recipe::ResourceAmount> = recipe
                        .outputs
                        .iter()
                        .map(|o| super::recipe::ResourceAmount {
                            kind: o.kind.clone(),
                            amount: o.amount * batch_size,
                        })
                        .collect();
                    Self::produce_outputs(stock_map, &multiplied_outputs);
                    job["state"] = json!("complete");

                    // Emit production_completed event
                    let outputs_payload: Vec<serde_json::Value> = recipe
                        .outputs
                        .iter()
                        .map(|o| {
                            json!({
                                "kind": o.kind,
                                "amount": o.amount * batch_size,
                            })
                        })
                        .collect();
                    let payload = json!({
                        "entity": eid,
                        "recipe": recipe_name,
                        "outputs": outputs_payload,
                        "batch_count": batch_size,
                    });
                    let _ = world.send_event("production_completed", payload);
                } else if job.get("assigned_to").and_then(|v| v.as_u64()).is_some() {
                    job["state"] = json!("in_progress");
                }
            } else {
                job["state"] = json!("waiting_for_inputs");
            }

            let _ = world.set_component(eid, "ProductionJob", job);
            let _ = world.set_component(eid, "Stockpile", stockpile);
        }
    }
}
