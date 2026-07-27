//! Unit template spawning: creates entities from data-driven templates.

use super::World;
use serde_json::{Map as JsonMap, Value as JsonValue};

impl World {
    /// Spawn an entity from a named template, applying optional component overrides.
    ///
    /// Returns the new entity ID.
    /// Errors if template not found or any `set_component` call fails.
    pub fn spawn_from_template(
        &mut self,
        template_name: &str,
        overrides: Option<JsonMap<String, JsonValue>>,
    ) -> Result<u32, String> {
        let template = self
            .template_registry
            .get_template(template_name)
            .ok_or_else(|| format!("Template '{}' not found", template_name))?
            .clone();

        let entity = self.spawn_entity();

        for (comp_name, comp_data) in &template.components {
            let mut final_data = comp_data.clone();

            // Deep-merge overrides if provided for this component
            if let Some(ref ov) = overrides
                && let Some(override_data) = ov.get(comp_name)
            {
                Self::deep_merge(&mut final_data, override_data);
            }

            self.set_component(entity, comp_name, final_data)?;
        }

        // Advisory: validate template equipment references against the item registry.
        // Warnings are logged but do not prevent the spawn from succeeding.
        if template.components.contains_key("Equipment") {
            let warnings =
                super::loadout::validate_template_equipment(&template, &self.item_registry);
            for w in &warnings {
                log::warn!(
                    "Template '{}': {} — entity spawned with incomplete equipment",
                    template.name,
                    w.message
                );
            }
        }

        Ok(entity)
    }

    /// Deep-merge `override_val` into `base_val` (for JSON objects).
    ///
    /// For non-object types, override replaces base entirely.
    pub fn deep_merge(base: &mut JsonValue, override_val: &JsonValue) {
        match (base, override_val) {
            (JsonValue::Object(base_map), JsonValue::Object(ov_map)) => {
                for (key, val) in ov_map {
                    Self::deep_merge(base_map.entry(key.clone()).or_insert(JsonValue::Null), val);
                }
            }
            (base, ov) => *base = ov.clone(),
        }
    }
}
