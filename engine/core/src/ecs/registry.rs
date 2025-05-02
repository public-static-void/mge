use super::{ComponentSchema, RegistryError};
use crate::ecs::error::MigrationError;
pub use semver::Version;
use serde_json;
use std::any::TypeId;

pub struct ComponentRegistry {
    components: std::collections::HashMap<TypeId, ComponentSchema>,
}

pub trait Component: 'static + Send + Sync {
    fn generate_schema() -> Option<schemars::schema::RootSchema>;

    fn version() -> Version {
        Version::parse("1.0.0").unwrap()
    }

    fn migrate(from_version: Version, data: &[u8]) -> Result<Self, MigrationError>
    where
        Self: Sized + serde::de::DeserializeOwned;
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: std::collections::HashMap::new(),
        }
    }

    pub fn register<T: super::Component>(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let type_id = TypeId::of::<T>();
        let schema = T::generate_schema();

        self.components.insert(
            type_id,
            ComponentSchema {
                name: std::any::type_name::<T>().to_string(),
                schema,
            },
        );
        Ok(())
    }

    pub fn get_schema<T: super::Component>(&self) -> Option<&ComponentSchema> {
        self.components.get(&TypeId::of::<T>())
    }

    pub fn schema_to_json<T: Component>(&self) -> Result<String, RegistryError> {
        let schema = self
            .get_schema::<T>()
            .ok_or(RegistryError::UnregisteredComponent)?;

        schema
            .schema
            .as_ref()
            .ok_or(RegistryError::InvalidSchema)
            .and_then(|s| serde_json::to_string_pretty(s).map_err(Into::into))
    }

    pub fn migrate_component<T: Component>(
        &self,
        data: &[u8],
        from_version: Version,
    ) -> Result<T, MigrationError>
    where
        T: serde::de::DeserializeOwned,
    {
        if from_version >= T::version() {
            return bson::from_slice(data).map_err(Into::into);
        }

        T::migrate(from_version, data)
    }
}
