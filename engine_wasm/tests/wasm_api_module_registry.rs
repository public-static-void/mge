//! ApiModule registry tests: the WASM bridge exposes every shared domain and
//! removing one registry entry unregisters that domain's host imports.

use engine_core::api_registry::SHARED_API_DOMAINS;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::worldgen::ThreadSafeWorldgenRegistry;
use engine_wasm::engine::{API_MODULES, ApiModule, WasmApiContext, api_module_names};
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Linker, Store};

/// Pinned WASM bridge module list, in registration order.
const EXPECTED_WASM_MODULES: &[&str] = &[
    "entity",
    "component",
    "construction",
    "turn",
    "mode",
    "death_decay",
    "dungeon",
    "time_of_day",
    "input",
    "inventory",
    "save_load",
    "camera",
    "event_bus",
    "system",
    "movement_ops",
    "equipment",
    "region",
    "body",
    "body_part_damage",
    "economic",
    "craft",
    "job_system",
    "job_board",
    "job_query",
    "job_mutation",
    "job_cancel",
    "job_events",
    "job_ai",
    "map",
    "multiscale_map",
    "world_userdata",
    "ui",
    "ui_tree",
    "ui_events",
    "loot",
    "material",
    "faction",
    "fov",
    "noise",
    "tech_tree",
    "unit_template",
    "vehicle",
    "weather",
    "temperature",
    "trade",
    "supply",
    "designer",
    "diplomacy",
    "narrative",
    "lore",
    "worldgen",
];

fn test_context() -> WasmApiContext {
    WasmApiContext {
        worldgen_registry: Arc::new(Mutex::new(ThreadSafeWorldgenRegistry::default())),
    }
}

type WasmLinker = Linker<Arc<Mutex<WasmWorld>>>;
type WasmStore = Store<Arc<Mutex<WasmWorld>>>;

fn import_present(linker: &WasmLinker, store: &mut WasmStore, module: &str, name: &str) -> bool {
    linker.get(store, module, name).is_ok()
}

fn register_into_fresh_linker(modules: &[&dyn ApiModule]) -> (WasmLinker, WasmStore) {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let ctx = test_context();
    engine_wasm::engine::register_wasm_apis(&mut linker, &ctx, modules)
        .expect("module registration failed");
    let world = Arc::new(Mutex::new(WasmWorld::new()));
    let store = Store::new(&engine, world);
    (linker, store)
}

#[test]
fn wasm_registry_matches_pinned_module_list() {
    assert_eq!(api_module_names(API_MODULES), EXPECTED_WASM_MODULES);
}

#[test]
fn wasm_registry_covers_all_shared_domains() {
    let names = api_module_names(API_MODULES);
    for domain in SHARED_API_DOMAINS {
        assert!(
            names.contains(domain),
            "shared domain '{domain}' missing from WASM API_MODULES"
        );
    }
}

#[test]
fn wasm_registry_names_are_unique() {
    let names = api_module_names(API_MODULES);
    let mut seen = std::collections::HashSet::new();
    for name in &names {
        assert!(seen.insert(*name), "duplicate module name '{name}'");
    }
}

#[test]
fn removing_a_module_unregisters_its_domain() {
    let (linker, mut store) = register_into_fresh_linker(API_MODULES);
    assert!(import_present(
        &linker,
        &mut store,
        "wasm_noise",
        "emit_noise"
    ));
    assert!(import_present(
        &linker,
        &mut store,
        "entity",
        "spawn_entity"
    ));

    let filtered: Vec<&dyn ApiModule> = API_MODULES
        .iter()
        .copied()
        .filter(|m| m.name() != "noise")
        .collect();
    assert_eq!(filtered.len(), API_MODULES.len() - 1);
    let (linker, mut store) = register_into_fresh_linker(&filtered);
    assert!(!import_present(
        &linker,
        &mut store,
        "wasm_noise",
        "emit_noise"
    ));
    assert!(!import_present(
        &linker,
        &mut store,
        "wasm_noise",
        "get_noise_at"
    ));
    assert!(import_present(
        &linker,
        &mut store,
        "entity",
        "spawn_entity"
    ));
}
