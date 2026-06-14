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
    fn generate_schema() -> Option<schemars::Schema>;

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
    pub fn register_component<T: super::Component>(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let type_id = TypeId::of::<T>();
        let schema = T::generate_schema();

        self.components.insert(
            type_id,
            ComponentSchema {
                name: std::any::type_name::<T>().to_string(),
                schema: schema
                    .map(|s| serde_json::to_value(&s).expect("failed to convert schema to JSON"))
                    .expect("schema must be present"),
                modes: vec![],
            },
        );
        Ok(())
    }

    /// Returns true if a component with the given name is registered
    pub fn is_registered(&self, name: &str) -> bool {
        self.get_schema_by_name(name).is_some()
    }

    /// Unregister a Rust-native component type by TypeId.
    pub fn unregister_component<T: super::Component>(&mut self) {
        let type_id = std::any::TypeId::of::<T>();
        self.components.remove(&type_id);
    }

    /// Get the schema for a registered component type.
    pub fn get_schema<T: super::Component>(&self) -> Option<&ComponentSchema> {
        self.components.get(&TypeId::of::<T>())
    }

    /// Get the schema for a registered component type by name.
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

    /// Return all component names registered in the registry
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

    /// Register an external component schema at runtime.
    pub fn register_external_schema(&mut self, schema: ComponentSchema) {
        self.external_components.insert(schema.name.clone(), schema);
    }

    /// Register an external component schema from a JSON string at runtime.
    pub fn register_external_schema_from_json(&mut self, json: &str) -> Result<()> {
        // Parse the JSON string into a serde_json::Value
        let v: serde_json::Value = serde_json::from_str(json)?;

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
            schema: v, // Store as serde_json::Value
            modes,
        };
        self.external_components.insert(name, cs);
        Ok(())
    }

    /// Return all component modes registered in the registry
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

    /// Get all component names registered for a given mode.
    pub fn components_for_mode(&self, mode: &str) -> Vec<String> {
        self.components
            .values()
            .chain(self.external_components.values())
            .filter(|schema| schema.modes.iter().any(|m| m == mode))
            .map(|schema| schema.name.clone())
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Health;
    use serde_json::json;

    fn empty_registry() -> ComponentRegistry {
        ComponentRegistry::new()
    }

    fn health_type_name() -> &'static str {
        std::any::type_name::<Health>()
    }

    // --- Registration / unregistration ---

    #[test]
    fn test_new_is_empty_len() {
        let reg = empty_registry();
        assert!(reg.all_component_names().is_empty());
        assert!(reg.all_modes().is_empty());
        assert!(!reg.is_registered("anything"));
    }

    #[test]
    fn test_register_component_roundtrip() {
        let mut reg = empty_registry();
        reg.register_component::<Health>().unwrap();
        let schema = reg.get_schema::<Health>();
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().name, health_type_name());
    }

    #[test]
    fn test_unregister_component() {
        let mut reg = empty_registry();
        reg.register_component::<Health>().unwrap();
        reg.unregister_component::<Health>();
        assert!(reg.get_schema::<Health>().is_none());
    }

    #[test]
    fn test_schema_to_json_unregistered() {
        let reg = empty_registry();
        let result = reg.schema_to_json::<Health>();
        assert!(matches!(result, Err(RegistryError::UnregisteredComponent)));
    }

    #[test]
    fn test_register_external_schema_roundtrip() {
        let mut reg = empty_registry();
        let schema = ComponentSchema {
            name: "TestComponent".into(),
            schema: json!({ "type": "object" }),
            modes: vec![],
        };
        reg.register_external_schema(schema);
        let retrieved = reg.get_schema_by_name("TestComponent");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestComponent");
    }

    #[test]
    fn test_unregister_external_schema() {
        let mut reg = empty_registry();
        let schema = ComponentSchema {
            name: "TestComponent".into(),
            schema: json!({}),
            modes: vec![],
        };
        reg.register_external_schema(schema);
        reg.unregister_external_schema("TestComponent");
        assert!(reg.get_schema_by_name("TestComponent").is_none());
    }

    #[test]
    fn test_register_external_from_json() {
        let mut reg = empty_registry();
        let json = r#"{"title": "FromJson", "type": "object", "modes": ["Colony"]}"#;
        reg.register_external_schema_from_json(json).unwrap();
        let schema = reg.get_schema_by_name("FromJson");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().name, "FromJson");
    }

    #[test]
    fn test_register_external_from_json_missing_title() {
        let mut reg = empty_registry();
        let json = r#"{"type": "object"}"#;
        let result = reg.register_external_schema_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_external_from_json_empty_string() {
        let mut reg = empty_registry();
        let result = reg.register_external_schema_from_json("");
        assert!(result.is_err());
    }

    // --- Error paths ---

    #[test]
    fn test_schema_to_json_empty_registry() {
        let reg = empty_registry();
        let result = reg.schema_to_json::<Health>();
        assert!(matches!(result, Err(RegistryError::UnregisteredComponent)));
    }

    // --- Query operations ---

    #[test]
    fn test_is_registered_native_and_external() {
        let mut reg = empty_registry();
        reg.register_component::<Health>().unwrap();
        let ext = ComponentSchema {
            name: "External".into(),
            schema: json!({}),
            modes: vec![],
        };
        reg.register_external_schema(ext);

        assert!(reg.is_registered(health_type_name()));
        assert!(reg.is_registered("External"));
        assert!(!reg.is_registered("NonExistent"));
    }

    #[test]
    fn test_all_component_names() {
        let mut reg = empty_registry();
        reg.register_component::<Health>().unwrap();
        let ext = ComponentSchema {
            name: "ExternalComp".into(),
            schema: json!({}),
            modes: vec![],
        };
        reg.register_external_schema(ext);

        let names = reg.all_component_names();
        assert!(names.contains(&health_type_name().to_string()));
        assert!(names.contains(&"ExternalComp".to_string()));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_all_modes_deduplicated() {
        let mut reg = empty_registry();
        let schema_a = ComponentSchema {
            name: "A".into(),
            schema: json!({}),
            modes: vec!["Colony".into(), "Roguelike".into()],
        };
        let schema_b = ComponentSchema {
            name: "B".into(),
            schema: json!({}),
            modes: vec!["Colony".into()],
        };
        reg.register_external_schema(schema_a);
        reg.register_external_schema(schema_b);

        let modes = reg.all_modes();
        assert!(modes.contains("Colony"));
        assert!(modes.contains("Roguelike"));
        assert_eq!(modes.len(), 2);
    }

    #[test]
    fn test_components_for_mode() {
        let mut reg = empty_registry();
        let schema = ComponentSchema {
            name: "ModeComp".into(),
            schema: json!({}),
            modes: vec!["Dungeon".into()],
        };
        reg.register_external_schema(schema);

        let matched = reg.components_for_mode("Dungeon");
        assert_eq!(matched, vec!["ModeComp"]);

        let no_match = reg.components_for_mode("Nonexistent");
        assert!(no_match.is_empty());
    }

    #[test]
    fn test_components_for_mode_no_match() {
        let reg = empty_registry();
        let result = reg.components_for_mode("MissingMode");
        assert!(result.is_empty());
    }

    // --- Update / hot-reload ---

    #[test]
    fn test_update_external_schema_replace() {
        let mut reg = empty_registry();
        let original = ComponentSchema {
            name: "Updatable".into(),
            schema: json!({ "version": 1 }),
            modes: vec![],
        };
        reg.register_external_schema(original);

        let updated = ComponentSchema {
            name: "Updatable".into(),
            schema: json!({ "version": 2 }),
            modes: vec!["NewMode".into()],
        };
        reg.update_external_schema(updated).unwrap();

        let retrieved = reg.get_schema_by_name("Updatable").unwrap();
        assert_eq!(retrieved.schema, json!({ "version": 2 }));
        assert_eq!(retrieved.modes, vec!["NewMode"]);
    }

    #[test]
    fn test_update_external_schema_insert() {
        let mut reg = empty_registry();
        let schema = ComponentSchema {
            name: "NewInsert".into(),
            schema: json!({ "key": "value" }),
            modes: vec![],
        };
        reg.update_external_schema(schema).unwrap();
        assert!(reg.get_schema_by_name("NewInsert").is_some());
    }

    #[test]
    fn test_update_external_schema_with_migration() {
        let mut reg = empty_registry();
        let schema = ComponentSchema {
            name: "Migrated".into(),
            schema: json!({ "version": 2 }),
            modes: vec![],
        };
        let mut data: std::collections::HashMap<u32, serde_json::Value> =
            [(1u32, json!({ "val": 10 })), (2u32, json!({ "val": 20 }))]
                .into_iter()
                .collect();

        reg.update_external_schema_with_migration(schema, &mut data, |v| {
            let mut map = v.as_object().unwrap().clone();
            map.insert("migrated".into(), json!(true));
            serde_json::Value::Object(map)
        })
        .unwrap();

        assert_eq!(data.get(&1u32).unwrap()["migrated"], json!(true));
        assert_eq!(data.get(&2u32).unwrap()["migrated"], json!(true));
        assert!(reg.get_schema_by_name("Migrated").is_some());
    }

    // --- Edge cases ---

    #[test]
    fn test_unregister_component_never_registered() {
        let mut reg = empty_registry();
        reg.unregister_component::<Health>();
        // Should not panic
    }

    #[test]
    fn test_unregister_external_schema_never_registered() {
        let mut reg = empty_registry();
        reg.unregister_external_schema("NeverRegistered");
        // Should not panic
    }

    #[test]
    fn test_register_external_schema_overwrite() {
        let mut reg = empty_registry();
        let first = ComponentSchema {
            name: "Overwrite".into(),
            schema: json!({ "data": "first" }),
            modes: vec![],
        };
        reg.register_external_schema(first);
        let second = ComponentSchema {
            name: "Overwrite".into(),
            schema: json!({ "data": "second" }),
            modes: vec![],
        };
        reg.register_external_schema(second);

        let retrieved = reg.get_schema_by_name("Overwrite").unwrap();
        assert_eq!(retrieved.schema, json!({ "data": "second" }));
    }

    #[test]
    fn test_get_schema_by_name_empty_registry() {
        let reg = empty_registry();
        assert!(reg.get_schema_by_name("Anything").is_none());
    }
}
