use super::World;
use crate::ecs::error::RegistryError;
use crate::ecs::schema::ComponentSchema;
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
        self.dynamic_systems.unregister_system(name);
    }

    /// Hot-reload a component schema in the registry.
    pub fn hotreload_schema(&mut self, schema: ComponentSchema) -> Result<(), RegistryError> {
        self.registry.lock().unwrap().update_external_schema(schema)
    }

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
