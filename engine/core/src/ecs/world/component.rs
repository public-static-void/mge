use super::World;
use crate::ecs::error::RegistryError;
use crate::ecs::schema::ComponentSchema;
use serde_json::Value as JsonValue;
use serde_json::json;

impl World {
    /// Sets a component value for an entity, validating against its schema if present,
    /// and emits a component_changed event.
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
            // Directly use the schema as serde_json::Value
            let validator = jsonschema::validator_for(&schema.schema)
                .map_err(|e| format!("Schema compile error: {e}"))?;
            let mut errors = validator.iter_errors(&value);
            if let Some(first_error) = errors.next() {
                // Collect all error messages
                let mut msgs = vec![first_error.to_string()];
                msgs.extend(errors.map(|e| e.to_string()));
                let msg = msgs.join(", ");
                return Err(format!("Schema validation failed: {msg}"));
            }
        }

        // Save old value for event emission
        let old = self
            .components
            .get(name)
            .and_then(|m| m.get(&entity))
            .cloned();

        self.components
            .entry(name.to_string())
            .or_default()
            .insert(entity, value.clone());

        // Emit component_changed event
        self.send_event(
            "component_changed",
            json!({
                "entity": entity,
                "component": name,
                "action": "set",
                "old": old,
                "new": value
            }),
        )
        .ok();

        Ok(())
    }

    /// Gets a reference to a component value for an entity.
    pub fn get_component(&self, entity: u32, name: &str) -> Option<&JsonValue> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return None;
        }
        self.components.get(name)?.get(&entity)
    }

    /// Removes a component from an entity, enforcing mode restrictions and emitting a component_changed event.
    pub fn remove_component(&mut self, entity: u32, name: &str) -> Result<(), String> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return Err(format!(
                "Component {} not allowed in mode {}",
                name, self.current_mode
            ));
        }
        let old = self
            .components
            .get_mut(name)
            .and_then(|m| m.remove(&entity));
        if old.is_some() {
            // Emit component_changed event
            self.send_event(
                "component_changed",
                json!({
                    "entity": entity,
                    "component": name,
                    "action": "removed",
                    "old": old,
                    "new": null
                }),
            )
            .ok();
            Ok(())
        } else {
            Err("Component not found".to_string())
        }
    }

    /// Returns true if the component is allowed in the current mode.
    pub fn is_component_allowed_in_mode(&self, component: &str, mode: &str) -> bool {
        if let Some(schema) = self.registry.lock().unwrap().get_schema_by_name(component) {
            schema.modes.contains(&mode.to_string())
        } else {
            false
        }
    }

    /// Unregister a component schema and remove all component data of that type.
    pub fn unregister_component_and_cleanup(&mut self, name: &str) {
        self.registry
            .lock()
            .unwrap()
            .unregister_external_schema(name);
        self.components.remove(name);
    }

    /// Unregister a dynamic system by name.
    pub fn unregister_dynamic_system(&mut self, name: &str) {
        let _ = self.dynamic_systems.unregister_system(name);
    }

    /// Hot-reload a component schema in the registry.
    pub fn hotreload_schema(&mut self, schema: ComponentSchema) -> Result<(), RegistryError> {
        self.registry.lock().unwrap().update_external_schema(schema)
    }

    /// Hot-reload a component schema and migrate component data.
    pub fn hotreload_schema_with_migration<F>(
        &mut self,
        schema: ComponentSchema,
        migrate: F,
    ) -> Result<(), RegistryError>
    where
        F: Fn(&serde_json::Value) -> serde_json::Value,
    {
        if let Some(data) = self.components.get_mut(&schema.name) {
            self.registry
                .lock()
                .unwrap()
                .update_external_schema_with_migration(schema, data, migrate)
        } else {
            if let Err(e) = self.registry.lock().unwrap().update_external_schema(schema) {
                // Handle or log the error as appropriate
                eprintln!("Failed to update schema: {:?}", e);
            }
            Ok(())
        }
    }
}
