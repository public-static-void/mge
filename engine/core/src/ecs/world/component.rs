use super::World;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value as JsonValue;

impl World {
    pub fn set_component(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return Err(format!(
                "Component {} not allowed in mode {}",
                name, self.current_mode
            ));
        }

        if let Some(schema) = self.registry.lock().unwrap().get_schema_by_name(name) {
            let compiled = JSONSchema::options()
                .with_draft(Draft::Draft7)
                .compile(&serde_json::to_value(&schema.schema).unwrap())
                .map_err(|e| format!("Schema compile error: {e}"))?;
            let result = compiled.validate(&value);
            if let Err(errors) = result {
                let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                return Err(format!("Schema validation failed: {msg}"));
            }
        }

        self.components
            .entry(name.to_string())
            .or_default()
            .insert(entity, value);
        Ok(())
    }

    pub fn get_component(&self, entity: u32, name: &str) -> Option<&JsonValue> {
        self.components.get(name)?.get(&entity)
    }

    pub fn is_component_allowed_in_mode(&self, component: &str, mode: &str) -> bool {
        if let Some(schema) = self.registry.lock().unwrap().get_schema_by_name(component) {
            schema.modes.contains(&mode.to_string())
        } else {
            false
        }
    }
}
