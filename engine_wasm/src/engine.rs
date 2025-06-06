use crate::host_api::entity::register_entity_api;
use anyhow::{Context, Result};
use engine_core::ecs::world::wasm::WasmWorld;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Instance, Linker, Module, Store, Val};

pub type HostImportRegistrar = Box<dyn Fn(&mut Linker<Arc<Mutex<WasmWorld>>>) + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
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

pub struct WasmScriptEngineConfig {
    pub module_path: PathBuf,
    pub import_host_functions: Option<HostImportRegistrar>,
}

pub struct WasmScriptEngine {
    store: Mutex<Store<Arc<Mutex<WasmWorld>>>>,
    instance: Instance,
}

impl WasmScriptEngine {
    pub fn new(config: WasmScriptEngineConfig) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, &config.module_path)
            .with_context(|| format!("Failed to load WASM module: {:?}", config.module_path))?;

        let mut linker = Linker::new(&engine);
        register_entity_api(&mut linker)?;

        if let Some(imports) = config.import_host_functions {
            imports(&mut linker);
        }

        let world = Arc::new(Mutex::new(WasmWorld::new()));
        let mut store = Store::new(&engine, world.clone());
        let instance = linker
            .instantiate(&mut store, &module)
            .context("Failed to instantiate WASM module")?;

        Ok(Self {
            store: Mutex::new(store),
            instance,
        })
    }

    pub fn invoke_exported_function(
        &self,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Option<WasmValue>> {
        let mut store_guard = self.store.lock().unwrap();

        let func = self
            .instance
            .get_func(&mut *store_guard, func_name)
            .with_context(|| format!("Exported function '{}' not found", func_name))?;

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
            Ok(Some(WasmValue::try_from(results[0].clone())?))
        }
    }
}
