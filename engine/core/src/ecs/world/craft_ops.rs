//! World-level crafting operations backing the Lua/Python/WASM bridges.
//!
//! Errors are `Result`-style strings shared byte-identically across bridges:
//! `"unknown_recipe"`, `"already_crafting"`, `"missing_tool:<item>"`,
//! `"missing_material:<material>"`, `"missing_input:<kind>"`,
//! `"insufficient_skill"`, `"no_craft_order"`.

use super::World;
use crate::systems::crafting::{consume_craft_inputs, validate_craft};
use serde_json::{Value as JsonValue, json};

impl World {
    /// Register a craft recipe under `name` from its JSON definition.
    ///
    /// The JSON is validated through the extended `Recipe` loader (all new
    /// craft fields optional), so existing stockpile-only files parse
    /// unchanged. Stockpile-only recipes (no `output_item`) are stored but
    /// stay unknown to the craft path: `can_craft`/`start_craft` report
    /// `unknown_recipe` for them.
    pub fn register_craft_recipe(
        &mut self,
        name: String,
        recipe_json: JsonValue,
    ) -> Result<(), String> {
        let recipe: crate::systems::economic::recipe::Recipe =
            serde_json::from_value(recipe_json).map_err(|e| format!("invalid_recipe: {e}"))?;
        self.craft_recipes.insert(name, recipe);
        Ok(())
    }

    /// Names of registered craft-path recipes (those with `output_item`),
    /// sorted ascending.
    pub fn list_craft_recipes(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .craft_recipes
            .iter()
            .filter(|(_, recipe)| recipe.output_item.is_some())
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// Pure gate check: `Ok(())` when `crafter` may start `recipe`, else the
    /// exact error string (`unknown_recipe`, `already_crafting`,
    /// `missing_input:<kind>`, `missing_material:<material>`,
    /// `missing_tool:<item>`, `insufficient_skill`). Never mutates.
    pub fn can_craft(&self, crafter: u32, recipe: &str) -> Result<(), String> {
        validate_craft(self, crafter, recipe).map(|_| ())
    }

    /// Start crafting: runs the shared gate, replaces any terminal order,
    /// creates an in-progress `CraftOrder`, then deducts inputs/materials
    /// from `Stockpile.resources` and removes consumed tools once.
    pub fn start_craft(&mut self, crafter: u32, recipe: &str) -> Result<(), String> {
        let recipe = validate_craft(self, crafter, recipe)?;
        if self
            .get_component(crafter, "CraftOrder")
            .and_then(|order| order.get("state"))
            .and_then(|state| state.as_str())
            != Some("in_progress")
        {
            let _ = self.remove_component(crafter, "CraftOrder");
        }
        let order = json!({"recipe": recipe.name, "progress": 0, "state": "in_progress", "output_entity": null});
        self.set_component(crafter, "CraftOrder", order)
            .map_err(|e| e.to_string())?;
        consume_craft_inputs(self, crafter, &recipe);
        Ok(())
    }

    /// Clone of the crafter's `CraftOrder`, or `None` when absent.
    pub fn get_craft_state(&self, crafter: u32) -> Option<JsonValue> {
        self.get_component(crafter, "CraftOrder").cloned()
    }

    /// Cancel an in-progress order: refunds stockpile inputs/materials
    /// (consumable tools are not refunded), removes the order, and emits
    /// `craft_cancelled { entity, recipe, refunded: true }`. Terminal or
    /// missing orders report `no_craft_order`.
    pub fn cancel_craft(&mut self, crafter: u32) -> Result<bool, String> {
        let order = self
            .get_component(crafter, "CraftOrder")
            .cloned()
            .ok_or_else(|| "no_craft_order".to_string())?;
        if order.get("state").and_then(|state| state.as_str()) != Some("in_progress") {
            return Err("no_craft_order".to_string());
        }
        let recipe_name = order
            .get("recipe")
            .and_then(|recipe| recipe.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(recipe) = self.craft_recipes.get(&recipe_name).cloned()
            && self.has_component(crafter, "Stockpile")
            && let Some(stockpile) = self.get_component(crafter, "Stockpile").cloned()
        {
            let mut updated = stockpile.clone();
            if let Some(map) = updated
                .get_mut("resources")
                .and_then(|resources| resources.as_object_mut())
            {
                for input in &recipe.inputs {
                    let current = map
                        .get(&input.kind)
                        .map(|count| {
                            count
                                .as_i64()
                                .unwrap_or_else(|| count.as_f64().unwrap_or(0.0) as i64)
                        })
                        .unwrap_or(0);
                    map.insert(input.kind.clone(), json!(current + input.amount));
                }
                for material in &recipe.materials {
                    let current = map
                        .get(&material.material)
                        .map(|count| {
                            count
                                .as_i64()
                                .unwrap_or_else(|| count.as_f64().unwrap_or(0.0) as i64)
                        })
                        .unwrap_or(0);
                    map.insert(material.material.clone(), json!(current + material.amount));
                }
            }
            let _ = self.set_component(crafter, "Stockpile", updated);
        }
        self.remove_component(crafter, "CraftOrder")
            .map_err(|_| "no_craft_order".to_string())?;
        let _ = self.send_event(
            "craft_cancelled",
            json!({"entity": crafter, "recipe": recipe_name, "refunded": true}),
        );
        Ok(true)
    }
}
