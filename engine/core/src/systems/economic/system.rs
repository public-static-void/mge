use crate::World;
use crate::ecs::system::System;
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

use super::recipe::{Recipe, ResourceAmount};

#[derive(Default)]
pub struct EconomicSystem {
    recipes: HashMap<String, Recipe>,
}

impl EconomicSystem {
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

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let entities = world.get_entities_with_component("ProductionJob");
        for eid in entities {
            let mut job = world.get_component(eid, "ProductionJob").unwrap().clone();
            let state = job
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");

            // Only skip if complete; process pending and in_progress
            if state == "complete" {
                continue;
            }

            // Allow pending jobs to start
            if state == "pending" {
                job["state"] = json!("in_progress");
            }

            let recipe_name = job.get("recipe").and_then(|v| v.as_str()).unwrap();
            let recipe = match self.recipes.get(recipe_name) {
                Some(r) => r,
                None => continue,
            };
            let mut stockpile = world.get_component(eid, "Stockpile").unwrap().clone();
            let stock_map = stockpile
                .get_mut("resources")
                .and_then(|v| v.as_object_mut())
                .unwrap();

            if Self::can_consume_inputs(stock_map, &recipe.inputs) {
                Self::consume_inputs(stock_map, &recipe.inputs);

                let progress = job.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
                job["progress"] = json!(progress);

                if progress >= recipe.duration {
                    Self::produce_outputs(stock_map, &recipe.outputs);
                    job["state"] = json!("complete");
                } else {
                    job["state"] = json!("in_progress");
                }
            } else {
                job["state"] = json!("waiting_for_inputs");
            }

            world.set_component(eid, "ProductionJob", job).unwrap();
            world.set_component(eid, "Stockpile", stockpile).unwrap();
        }
    }
}
