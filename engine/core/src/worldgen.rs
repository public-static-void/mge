use crate::scripting::helpers::{json_to_lua_table, lua_value_to_json};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::Value as JsonValue;
use std::fmt;

#[derive(Debug)]
pub enum WorldgenError {
    NotFound,
    LuaError(mlua::Error),
}

impl fmt::Display for WorldgenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WorldgenError {}

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
        params: &JsonValue,
    ) -> Result<JsonValue, WorldgenError> {
        for plugin in &self.plugins {
            if let WorldgenPlugin::Lua {
                name: plugin_name,
                registry_key,
            } = plugin
            {
                if plugin_name == name {
                    let func: mlua::Function = lua
                        .registry_value(registry_key)
                        .map_err(WorldgenError::LuaError)?;
                    let params_table =
                        json_to_lua_table(lua, params).map_err(WorldgenError::LuaError)?;
                    let result: mlua::Value =
                        func.call(params_table).map_err(WorldgenError::LuaError)?;
                    return lua_value_to_json(lua, result, None).map_err(WorldgenError::LuaError);
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

/// Registers built-in worldgen plugins with the registry.
pub fn register_builtin_worldgen_plugins(registry: &mut WorldgenRegistry) {
    registry.register(WorldgenPlugin::CAbi {
        name: "basic_square_worldgen".to_string(),
        generate: Box::new(|params: &JsonValue| {
            let width = params.get("width").and_then(|v| v.as_u64()).unwrap() as i32;
            let height = params.get("height").and_then(|v| v.as_u64()).unwrap() as i32;
            let z_levels = params.get("z_levels").and_then(|v| v.as_u64()).unwrap() as i32;
            let seed = params.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
            let biomes = params
                .get("biomes")
                .and_then(|v| v.as_array())
                .cloned()
                .filter(|arr| !arr.is_empty())
                .unwrap_or_else(|| {
                    vec![serde_json::json!({"name": "Default", "tiles": ["default"]})]
                });

            let biome_names: Vec<&str> = biomes
                .iter()
                .filter_map(|b| b.get("name").and_then(|n| n.as_str()))
                .collect();
            let biome_tiles: Vec<Vec<&str>> = biomes
                .iter()
                .map(|b| {
                    b.get("tiles")
                        .and_then(|t| t.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default()
                })
                .collect();

            let mut rng = StdRng::seed_from_u64(seed);

            let mut cells = Vec::new();
            for z in 0..z_levels {
                for y in 0..height {
                    for x in 0..width {
                        // Pick a biome and terrain randomly (deterministic via seed)
                        let biome_idx = rng.random_range(0..biome_names.len());
                        let biome = biome_names[biome_idx];
                        let tiles = &biome_tiles[biome_idx];
                        let terrain = if !tiles.is_empty() {
                            tiles[rng.random_range(0..tiles.len())]
                        } else {
                            "unknown"
                        };

                        // --- Neighbor calculation ---
                        let mut neighbors = Vec::new();
                        let deltas = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                        for (dx, dy) in deltas.iter() {
                            let nx = x + dx;
                            let ny = y + dy;
                            if nx >= 0 && nx < width && ny >= 0 && ny < height {
                                neighbors.push(serde_json::json!({
                                    "x": nx,
                                    "y": ny,
                                    "z": z
                                }));
                            }
                        }

                        cells.push(serde_json::json!({
                            "x": x,
                            "y": y,
                            "z": z,
                            "biome": biome,
                            "terrain": terrain,
                            "neighbors": neighbors
                        }));
                    }
                }
            }
            serde_json::json!({
                "topology": "square",
                "cells": cells
            })
        }),
    });
}
