use crate::ecs::error::{MigrationError, RegistryError};
use crate::ecs::schema::ComponentSchema;
use anyhow::Result;
pub use semver::Version;
use serde_json;
use std::any::TypeId;
use std::collections::HashMap;

/// Registry for component schemas and metadata.
pub struct ComponentRegistry {
    components: HashMap<TypeId, ComponentSchema>,
    external_components: HashMap<String, ComponentSchema>,
}

/// Trait for ECS components supporting schema, versioning, and migration.
pub trait Component: 'static + Send + Sync {
    /// Generate a JSON schema for this component.
    fn generate_schema() -> Option<schemars::schema::RootSchema>;

    /// Return the component's version.
    fn version() -> Version {
        Version::parse("1.0.0").unwrap()
    }

    /// Migrate component data from an older version.
    fn migrate(from_version: Version, data: &[u8]) -> Result<Self, MigrationError>
    where
        Self: Sized + serde::de::DeserializeOwned;
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            external_components: HashMap::new(),
        }
    }

    /// Register a component type and its schema.
    pub fn register<T: super::Component>(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let type_id = TypeId::of::<T>();
        let schema = T::generate_schema();

        self.components.insert(
            type_id,
            ComponentSchema {
                name: std::any::type_name::<T>().to_string(),
                schema: schema.expect("schema must be present"),
                modes: vec![],
            },
        );
        Ok(())
    }

    /// Get the schema for a registered component type.
    pub fn get_schema<T: super::Component>(&self) -> Option<&ComponentSchema> {
        self.components.get(&TypeId::of::<T>())
    }

    pub fn get_schema_by_name(&self, name: &str) -> Option<&ComponentSchema> {
        self.components
            .values()
            .find(|schema| schema.name == name)
            .or_else(|| self.external_components.get(name))
    }

    /// Get the JSON schema for a component as a pretty-printed string.
    pub fn schema_to_json<T: Component>(&self) -> Result<String, RegistryError> {
        let schema = self
            .get_schema::<T>()
            .ok_or(RegistryError::UnregisteredComponent)?;

        serde_json::to_string_pretty(&schema.schema).map_err(Into::into)
    }

    /// Migrate component data from a previous version.
    pub fn migrate_component<T>(
        &self,
        data: &[u8],
        from_version: Version,
    ) -> Result<T, MigrationError>
    where
        T: Component + serde::de::DeserializeOwned,
    {
        if from_version >= T::version() {
            return bson::from_slice(data).map_err(Into::into);
        }

        T::migrate(from_version, data)
    }

    pub fn all_component_names(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for schema in self.components.values() {
            names.insert(schema.name.clone());
        }
        for schema in self.external_components.values() {
            names.insert(schema.name.clone());
        }
        names.into_iter().collect()
    }

    pub fn register_external_schema(&mut self, schema: ComponentSchema) {
        self.external_components.insert(schema.name.clone(), schema);
    }

    /// Register an external component schema from a JSON string at runtime.
    pub fn register_external_schema_from_json(&mut self, json: &str) -> Result<()> {
        // Parse the JSON string into a serde_json::Value
        let v: serde_json::Value = serde_json::from_str(json)?;

        // Parse as RootSchema for validation/storage
        let schema: schemars::schema::RootSchema = serde_json::from_value(v.clone())?;

        // Extract title (name)
        let name = v
            .get("title")
            .and_then(|t| t.as_str())
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("Missing 'title' in schema"))?;

        // Extract modes
        let modes = v
            .get("modes")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        // Insert into registry
        let cs = ComponentSchema {
            name: name.clone(),
            schema,
            modes,
        };
        self.external_components.insert(name, cs);
        Ok(())
    }

    pub fn all_modes(&self) -> std::collections::HashSet<String> {
        let mut modes = std::collections::HashSet::new();
        for schema in self
            .components
            .values()
            .chain(self.external_components.values())
        {
            for mode in &schema.modes {
                modes.insert(mode.clone());
            }
        }
        modes
    }

    /// Unregister an external component schema by name.
    pub fn unregister_external_schema(&mut self, name: &str) {
        self.external_components.remove(name);
    }

    /// Update (hot-reload) an external component schema by name.
    /// If the schema exists, it is replaced. If not, it is registered anew.
    pub fn update_external_schema(&mut self, schema: ComponentSchema) -> Result<(), RegistryError> {
        self.external_components.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// Update (hot-reload) an external component schema by name, migrating all data.
    pub fn update_external_schema_with_migration<F>(
        &mut self,
        schema: ComponentSchema,
        component_data: &mut std::collections::HashMap<u32, serde_json::Value>,
        migrate: F,
    ) -> Result<(), RegistryError>
    where
        F: Fn(&serde_json::Value) -> serde_json::Value,
    {
        for value in component_data.values_mut() {
            *value = migrate(value);
        }
        self.external_components.insert(schema.name.clone(), schema);
        Ok(())
    }
}
