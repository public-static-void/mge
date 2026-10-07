//! ApiModule registry tests: the Lua bridge exposes every shared domain and
//! removing one registry entry unregisters that domain.

use engine_core::api_registry::SHARED_API_DOMAINS;
use engine_core::ecs::World;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::worldgen::WorldgenRegistry;
use engine_lua::input::{InputProvider, StdinInput};
use engine_lua::lua_api::{API_MODULES, ApiModule, LuaApiContext, api_module_names};
use mlua::{Lua, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Pinned Lua bridge module list, in registration order.
const EXPECTED_LUA_MODULES: &[&str] = &[
    "world",
    "worldgen",
    "event_bus",
    "system",
    "entity",
    "component",
    "input",
    "inventory",
    "equipment",
    "body",
    "region",
    "camera",
    "ui",
    "mode",
    "turn",
    "save_load",
    "death_decay",
    "time_of_day",
    "map",
    "multiscale_map",
    "narrative",
    "lore",
    "economic",
    "construction",
    "craft",
    "movement_ops",
    "dungeon",
    "diplomacy",
    "trade",
    "supply",
    "job_ai",
    "loot",
    "faction",
    "material",
    "tech_tree",
    "fov",
    "noise",
    "ai_behaviors",
    "unit_template",
    "vehicle",
    "item_definition",
    "equipment_set_designer",
    "weather",
    "temperature",
    "job_system",
    "job_board",
    "job_query",
    "job_mutation",
    "job_cancel",
    "job_events",
];

fn test_context(lua: Rc<Lua>) -> LuaApiContext {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world = Rc::new(RefCell::new(World::new(registry)));
    LuaApiContext {
        world,
        input_provider: Arc::new(Mutex::new(
            Box::new(StdinInput) as Box<dyn InputProvider + Send + Sync>
        )),
        worldgen_registry: Rc::new(RefCell::new(WorldgenRegistry::new())),
        lua: Rc::clone(&lua),
        lua_systems: Rc::new(RefCell::new(HashMap::new())),
    }
}

fn register_modules(modules: &[&dyn ApiModule]) -> Rc<Lua> {
    let lua = Rc::new(Lua::new());
    let globals = lua.globals();
    let ctx = test_context(Rc::clone(&lua));
    engine_lua::lua_api::register_api_modules(&lua, &globals, &ctx, modules)
        .expect("module registration failed");
    lua
}

#[test]
fn lua_registry_matches_pinned_module_list() {
    assert_eq!(api_module_names(API_MODULES), EXPECTED_LUA_MODULES);
}

#[test]
fn lua_registry_covers_all_shared_domains() {
    let names = api_module_names(API_MODULES);
    for domain in SHARED_API_DOMAINS {
        assert!(
            names.contains(domain),
            "shared domain '{domain}' missing from Lua API_MODULES"
        );
    }
}

#[test]
fn lua_registry_names_are_unique() {
    let names = api_module_names(API_MODULES);
    let mut seen = std::collections::HashSet::new();
    for name in &names {
        assert!(seen.insert(*name), "duplicate module name '{name}'");
    }
}

#[test]
fn removing_a_module_unregisters_its_domain() {
    let lua = register_modules(API_MODULES);
    let globals = lua.globals();
    assert!(matches!(
        globals.get::<Value>("emit_noise"),
        Ok(Value::Function(_))
    ));
    assert!(matches!(
        globals.get::<Value>("spawn_entity"),
        Ok(Value::Function(_))
    ));

    let filtered: Vec<&dyn ApiModule> = API_MODULES
        .iter()
        .copied()
        .filter(|m| m.name() != "noise")
        .collect();
    assert_eq!(filtered.len(), API_MODULES.len() - 1);
    let lua = register_modules(&filtered);
    let globals = lua.globals();
    assert!(matches!(globals.get::<Value>("emit_noise"), Ok(Value::Nil)));
    assert!(matches!(
        globals.get::<Value>("spawn_entity"),
        Ok(Value::Function(_))
    ));
}
