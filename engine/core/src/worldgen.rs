use crate::scripting::helpers::{json_to_lua_table, lua_value_to_json};
use serde_json::Value as JsonValue;

#[derive(Debug)]
pub enum WorldgenError {
    NotFound,
    LuaError(mlua::Error),
}

pub enum WorldgenPlugin {
    CAbi {
        name: String,
        generate: Box<dyn Fn(&JsonValue) -> JsonValue>,
    },
    Python {
        name: String,
        generate: Box<dyn Fn(&JsonValue) -> JsonValue>,
    },
    Lua {
        name: String,
        registry_key: mlua::RegistryKey,
    },
}

pub struct WorldgenRegistry {
    plugins: Vec<WorldgenPlugin>,
}

impl WorldgenRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn register(&mut self, plugin: WorldgenPlugin) {
        self.plugins.push(plugin);
    }

    pub fn list_names(&self) -> Vec<String> {
        self.plugins
            .iter()
            .map(|p| match p {
                WorldgenPlugin::CAbi { name, .. }
                | WorldgenPlugin::Python { name, .. }
                | WorldgenPlugin::Lua { name, .. } => name.clone(),
            })
            .collect()
    }

    pub fn invoke(&self, name: &str, params: &JsonValue) -> Result<JsonValue, WorldgenError> {
        for plugin in &self.plugins {
            let plugin_name = match plugin {
                WorldgenPlugin::CAbi { name, .. }
                | WorldgenPlugin::Python { name, .. }
                | WorldgenPlugin::Lua { name, .. } => name,
            };
            if plugin_name == name {
                let generate = match plugin {
                    WorldgenPlugin::CAbi { generate, .. }
                    | WorldgenPlugin::Python { generate, .. } => generate,
                    WorldgenPlugin::Lua { .. } => {
                        return Err(WorldgenError::NotFound);
                    }
                };
                return Ok(generate(params));
            }
        }
        Err(WorldgenError::NotFound)
    }

    pub fn invoke_lua(
        &self,
        lua: &mlua::Lua,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, WorldgenError> {
        for plugin in &self.plugins {
            if let WorldgenPlugin::Lua {
                name: plugin_name,
                registry_key,
            } = plugin
            {
                if plugin_name == name {
                    // 1. Retrieve the Lua function from the registry
                    let func: mlua::Function = lua
                        .registry_value(registry_key)
                        .map_err(WorldgenError::LuaError)?;
                    // 2. Convert params from JSON to Lua table
                    let params_table =
                        json_to_lua_table(lua, params).map_err(WorldgenError::LuaError)?;
                    // 3. Call the Lua function
                    let result: mlua::Value =
                        func.call(params_table).map_err(WorldgenError::LuaError)?;
                    // 4. Convert the result from Lua value to JSON
                    return lua_value_to_json(lua, result).map_err(WorldgenError::LuaError);
                }
            }
        }
        Err(WorldgenError::NotFound)
    }
}

impl Default for WorldgenRegistry {
    fn default() -> Self {
        Self::new()
    }
}
