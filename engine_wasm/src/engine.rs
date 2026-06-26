use crate::host_api::body::register_body_api;
use crate::host_api::camera::register_camera_api;
use crate::host_api::component::register_component_api;
use crate::host_api::death_decay::register_death_decay_api;
use crate::host_api::economic::register_economic_api;
use crate::host_api::entity::register_entity_api;
use crate::host_api::equipment::register_equipment_api;
use crate::host_api::event_bus::register_event_bus_api;
use crate::host_api::input::register_input_api;
use crate::host_api::inventory::register_inventory_api;
use crate::host_api::loot::register_loot_api;
use crate::host_api::job_ai::register_job_ai_api;
use crate::host_api::job_board::register_job_board_api;
use crate::host_api::job_cancel::register_job_cancel_api;
use crate::host_api::job_events::register_job_events_api;
use crate::host_api::job_mutation::register_job_mutation_api;
use crate::host_api::job_query::register_job_query_api;
use crate::host_api::job_system::register_job_system_api;
use crate::host_api::map::register_map_api;
use crate::host_api::mode::register_mode_api;
use crate::host_api::movement_ops::register_movement_ops_api;
use crate::host_api::region::register_region_api;
use crate::host_api::save_load::register_save_load_api;
use crate::host_api::system::register_system_api;
use crate::host_api::time_of_day::register_time_of_day_api;
use crate::host_api::turn::register_turn_api;
use crate::host_api::ui::register_ui_api;
use crate::host_api::ui_events::register_ui_events_api;
use crate::host_api::ui_tree::register_ui_tree_api;
use crate::host_api::world_userdata::register_world_userdata_api;
use crate::host_api::worldgen::register_worldgen_api;
use anyhow::Result;
use engine_core::ecs::world::wasm::{WasmWorld, load_schemas_from_dir};
use engine_core::worldgen::ThreadSafeWorldgenRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wasmtime::error::Context as WasmContext;
use wasmtime::{Engine, Extern, Func, Instance, Linker, Module, Store, Val};

/// A function to register host imports
pub type HostImportRegistrar = Box<dyn Fn(&mut Linker<Arc<Mutex<WasmWorld>>>) + Send + Sync>;

/// A value in the Wasm world
#[derive(Debug, Clone, PartialEq)]
pub enum WasmValue {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
}

impl From<i32> for WasmValue {
    fn from(v: i32) -> Self {
        WasmValue::I32(v)
    }
}
impl From<i64> for WasmValue {
    fn from(v: i64) -> Self {
        WasmValue::I64(v)
    }
}
impl From<f32> for WasmValue {
    fn from(v: f32) -> Self {
        WasmValue::F32(v)
    }
}
impl From<f64> for WasmValue {
    fn from(v: f64) -> Self {
        WasmValue::F64(v)
    }
}

impl From<WasmValue> for Val {
    fn from(v: WasmValue) -> Self {
        match v {
            WasmValue::I32(i) => Val::I32(i),
            WasmValue::I64(i) => Val::I64(i),
            WasmValue::F32(f) => Val::F32(f.to_bits()),
            WasmValue::F64(f) => Val::F64(f.to_bits()),
        }
    }
}

impl TryFrom<Val> for WasmValue {
    type Error = anyhow::Error;
    fn try_from(v: Val) -> Result<Self> {
        Ok(match v {
            Val::I32(i) => WasmValue::I32(i),
            Val::I64(i) => WasmValue::I64(i),
            Val::F32(f) => WasmValue::F32(f32::from_bits(f)),
            Val::F64(f) => WasmValue::F64(f64::from_bits(f)),
            _ => anyhow::bail!("Unsupported WASM value type"),
        })
    }
}

/// Known export name constants that the WASM guest may provide.
pub const EXPORT_WORLDGEN_GENERATE: &str = "mge_worldgen_generate";
pub const EXPORT_WORLDGEN_VALIDATE: &str = "mge_worldgen_validate";
pub const EXPORT_WORLDGEN_POSTPROCESS: &str = "mge_worldgen_postprocess";
pub const EXPORT_VALIDATE_MAP: &str = "mge_validate_map";
pub const EXPORT_POSTPROCESS_MAP: &str = "mge_postprocess_map";

/// Configuration for a WASM engine
pub struct WasmScriptEngineConfig {
    /// Path to the WASM module
    pub module_path: PathBuf,
    /// Optional path to schema directory (loads *.json files into WasmWorld.component_schemas)
    pub schema_path: Option<PathBuf>,
    /// Optional worldgen registry for list/invoke host functions
    pub worldgen_registry: Option<Arc<Mutex<ThreadSafeWorldgenRegistry>>>,
    /// Optional host function registrar
    pub import_host_functions: Option<HostImportRegistrar>,
}

/// A WASM script engine
pub struct WasmScriptEngine {
    store: Mutex<Store<Arc<Mutex<WasmWorld>>>>,
    instance: Instance,
    /// Discovered exports from the WASM module, keyed by export name.
    discovered_exports: HashMap<String, Func>,
}

impl WasmScriptEngine {
    /// Create a new WASM script engine
    pub fn new(config: WasmScriptEngineConfig) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, &config.module_path).map_err(|e| {
            e.context(format!(
                "Failed to load WASM module: {:?}",
                config.module_path
            ))
        })?;

        let mut linker = Linker::new(&engine);
        register_entity_api(&mut linker)?;
        register_component_api(&mut linker)?;
        register_turn_api(&mut linker)?;
        register_mode_api(&mut linker)?;
        register_death_decay_api(&mut linker)?;
        register_time_of_day_api(&mut linker)?;
        register_input_api(&mut linker)?;
        register_inventory_api(&mut linker)?;
        register_save_load_api(&mut linker)?;
        register_camera_api(&mut linker)?;
        register_event_bus_api(&mut linker)?;
        register_system_api(&mut linker)?;
        register_movement_ops_api(&mut linker)?;
        register_equipment_api(&mut linker)?;
        register_region_api(&mut linker)?;
        register_body_api(&mut linker)?;
        register_economic_api(&mut linker)?;
        register_job_system_api(&mut linker)?;
        register_job_board_api(&mut linker)?;
        register_job_query_api(&mut linker)?;
        register_job_mutation_api(&mut linker)?;
        register_job_cancel_api(&mut linker)?;
        register_job_ai_api(&mut linker)?;
        register_job_events_api(&mut linker)?;
        register_map_api(&mut linker)?;
        register_world_userdata_api(&mut linker)?;
        register_ui_api(&mut linker)?;
        register_ui_tree_api(&mut linker)?;
        register_ui_events_api(&mut linker)?;
        register_loot_api(&mut linker)?;

        // Load schemas if schema_path is provided
        let schemas = config
            .schema_path
            .as_ref()
            .map(|p| load_schemas_from_dir(p))
            .unwrap_or_default();

        // Register worldgen API (uses provided registry or empty default)
        let worldgen_registry = config
            .worldgen_registry
            .unwrap_or_else(|| Arc::new(Mutex::new(ThreadSafeWorldgenRegistry::default())));
        register_worldgen_api(&mut linker, worldgen_registry)?;

        if let Some(imports) = config.import_host_functions {
            imports(&mut linker);
        }

        let mut world = WasmWorld::new();
        world.component_schemas = schemas;
        let world = Arc::new(Mutex::new(world));
        let mut store = Store::new(&engine, world.clone());
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.context("Failed to instantiate WASM module"))?;

        // Scan exports for known function names and populate discovered_exports
        let discovered_exports: HashMap<String, Func> = instance
            .exports(&mut store)
            .filter_map(|export| {
                let name = export.name().to_string();
                match export.into_extern() {
                    Extern::Func(func) => Some((name, func)),
                    _ => None,
                }
            })
            .collect();

        // Store discovered export names in WasmWorld for host function access
        {
            let mut world_guard = world.lock().unwrap();
            world_guard.discovered_export_names = discovered_exports.keys().cloned().collect();
        }

        Ok(Self {
            store: Mutex::new(store),
            instance,
            discovered_exports,
        })
    }

    /// Invoke an exported function
    pub fn invoke_exported_function(
        &self,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Option<WasmValue>> {
        let mut store_guard = self.store.lock().unwrap();

        let func = self
            .instance
            .get_func(&mut *store_guard, func_name)
            .with_context(|| format!("Exported function '{func_name}' not found"))?;

        let ty = func.ty(&mut *store_guard);
        if ty.params().len() != args.len() {
            anyhow::bail!(
                "Function '{}' expects {} args, got {}",
                func_name,
                ty.params().len(),
                args.len()
            );
        }
        let vals: Vec<Val> = args.iter().cloned().map(Into::into).collect();

        let mut results = Vec::with_capacity(ty.results().len());
        for result in ty.results() {
            results.push(match result {
                wasmtime::ValType::I32 => Val::I32(0),
                wasmtime::ValType::I64 => Val::I64(0),
                wasmtime::ValType::F32 => Val::F32(0),
                wasmtime::ValType::F64 => Val::F64(0),
                _ => anyhow::bail!("Unsupported result type"),
            });
        }

        func.call(&mut *store_guard, &vals, &mut results)?;

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WasmValue::try_from(results[0])?))
        }
    }

    /// Call a discovered export by name, returning the first result value.
    /// The Func is cloned out of the map before locking the store to avoid
    /// simultaneous borrows of `self`.
    pub fn call_export(&self, name: &str, params: &[Val]) -> Result<Val, String> {
        let func = self
            .discovered_exports
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Export '{name}' not found"))?;

        let mut store_guard = self.store.lock().unwrap();
        let ty = func.ty(&mut *store_guard);

        let mut results: Vec<Val> = ty
            .results()
            .map(|rt| match rt {
                wasmtime::ValType::I32 => Val::I32(0),
                wasmtime::ValType::I64 => Val::I64(0),
                wasmtime::ValType::F32 => Val::F32(0),
                wasmtime::ValType::F64 => Val::F64(0),
                _ => Val::I32(0),
            })
            .collect();

        func.call(&mut *store_guard, params, &mut results)
            .map_err(|e| format!("Export '{}' call failed: {e}", name))?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| format!("Export '{name}' returned no results"))
    }
}
