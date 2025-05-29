use crate::ecs::World;
use crate::plugins::types::{EngineApi, PluginManifest, PluginVTable, SystemPlugin};
use crate::worldgen::{WorldgenPlugin, WorldgenRegistry};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use topo_sort::TopoSort;

/// # Safety
///
/// - `path` must point to a valid dynamic library exposing a compatible plugin vtable.
/// - `engine_api` and `world` must be valid for the duration of the plugin.
/// - The caller must ensure all pointer arguments are valid and not aliased elsewhere.
pub unsafe fn load_plugin<P: AsRef<Path>>(
    path: P,
    engine_api: &mut EngineApi,
    world: *mut c_void,
) -> Result<(), String> {
    let lib = unsafe { Library::new(path.as_ref()) }.map_err(|e| e.to_string())?;
    let vtable: Symbol<*mut *mut PluginVTable> =
        unsafe { lib.get(b"PLUGIN_VTABLE\0") }.map_err(|e| e.to_string())?;
    let plugin_vtable: *mut PluginVTable = unsafe { **vtable };
    if plugin_vtable.is_null() {
        return Err("PLUGIN_VTABLE symbol is null".to_string());
    }
    let vtable_ref: &PluginVTable = unsafe { &*plugin_vtable };

    let init_result = unsafe { (vtable_ref.init)(engine_api as *mut _, world) };
    if init_result != 0 {
        return Err(format!("Plugin init failed with code {}", init_result));
    }

    Ok(())
}

/// # Safety
///
/// - `path` must point to a valid dynamic library exposing a compatible plugin vtable.
/// - `engine_api`, `world`, and `worldgen_registry` must be valid for the duration of the plugin.
/// - The caller must ensure all pointer arguments are valid and not aliased elsewhere.
pub unsafe fn load_plugin_and_register_worldgen<P: AsRef<Path>>(
    path: P,
    engine_api: &mut EngineApi,
    world: *mut c_void,
    worldgen_registry: &mut WorldgenRegistry,
) -> Result<(), String> {
    let lib = unsafe { Library::new(path.as_ref()) }.map_err(|e| e.to_string())?;
    let vtable: Symbol<*mut *mut PluginVTable> =
        unsafe { lib.get(b"PLUGIN_VTABLE\0") }.map_err(|e| e.to_string())?;
    let plugin_vtable: *mut PluginVTable = unsafe { **vtable };
    if plugin_vtable.is_null() {
        return Err("PLUGIN_VTABLE symbol is null".to_string());
    }
    let vtable_ref: &PluginVTable = unsafe { &*plugin_vtable };

    let init_result = unsafe { (vtable_ref.init)(engine_api as *mut _, world) };
    if init_result != 0 {
        return Err(format!("Plugin init failed with code {}", init_result));
    }

    // --- Worldgen registration ---
    {
        let worldgen_name_fn = vtable_ref.worldgen_name;
        let generate_world_fn = vtable_ref.generate_world;
        let free_result_json_fn = vtable_ref.free_result_json;

        let name = if let Some(worldgen_name_fn) = worldgen_name_fn {
            let cstr = unsafe { CStr::from_ptr(worldgen_name_fn()) };
            cstr.to_str().unwrap().to_owned()
        } else {
            return Err("Plugin does not provide worldgen_name".to_string());
        };

        let generate = move |params: &Value| -> Value {
            let params_json = serde_json::to_string(params).unwrap();
            let c_params = CString::new(params_json).unwrap();
            let mut out_ptr: *mut c_char = std::ptr::null_mut();

            if let Some(generate_world_fn) = generate_world_fn {
                let res = unsafe { generate_world_fn(c_params.as_ptr(), &mut out_ptr) };
                if res != 0 || out_ptr.is_null() {
                    return Value::Null;
                }
                let result = unsafe { CStr::from_ptr(out_ptr).to_string_lossy().into_owned() };
                if let Some(free_result_json_fn) = free_result_json_fn {
                    unsafe { free_result_json_fn(out_ptr) };
                }
                serde_json::from_str(&result).unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        };

        worldgen_registry.register(WorldgenPlugin::CAbi {
            name,
            generate: Box::new(generate),
            _lib: Some(lib), // Dynamic plugin: keep the library alive
        });
    }

    Ok(())
}

/// # Safety
///
/// - `path` must point to a valid dynamic library exposing a compatible plugin vtable.
/// - `engine_api`, `world`, and `world_ref` must be valid for the duration of the plugin.
/// - The caller must ensure all pointer arguments are valid and not aliased elsewhere.
pub unsafe fn load_plugin_and_register_systems<P: AsRef<Path>>(
    path: P,
    engine_api: &mut EngineApi,
    world: *mut c_void,
    world_ref: &mut World,
) -> Result<(), String> {
    let lib = unsafe { Library::new(path.as_ref()) }.map_err(|e| e.to_string())?;
    let vtable: Symbol<*mut *mut PluginVTable> =
        unsafe { lib.get(b"PLUGIN_VTABLE\0") }.map_err(|e| e.to_string())?;
    let plugin_vtable: *mut PluginVTable = unsafe { **vtable };
    if plugin_vtable.is_null() {
        return Err("PLUGIN_VTABLE symbol is null".to_string());
    }
    let vtable_ref: &PluginVTable = unsafe { &*plugin_vtable };

    let init_result = unsafe { (vtable_ref.init)(engine_api as *mut _, world) };
    if init_result != 0 {
        return Err(format!("Plugin init failed with code {}", init_result));
    }

    // Register systems if available
    if let Some(register_systems_fn) = vtable_ref.register_systems {
        let mut systems_ptr: *mut SystemPlugin = std::ptr::null_mut();
        let mut count: c_int = 0;
        let res = unsafe {
            register_systems_fn(engine_api as *mut _, world, &mut systems_ptr, &mut count)
        };
        if res == 0 && !systems_ptr.is_null() && count > 0 {
            let systems_slice = unsafe { std::slice::from_raw_parts(systems_ptr, count as usize) };
            for sys in systems_slice {
                let name = unsafe { sys.name_str().to_string() };
                let run_fn = sys.run;
                // Wrap the C function pointer into a Rust closure
                let run_closure = Box::new(move |world: &mut World, delta_time: f32| unsafe {
                    run_fn(world as *mut _ as *mut std::ffi::c_void, delta_time)
                });
                world_ref.register_dynamic_system(&name, run_closure);
            }
            // Free systems_ptr if plugin allocated it dynamically
            if let Some(free_systems_fn) = vtable_ref.free_systems {
                unsafe { free_systems_fn(systems_ptr, count) };
            }
        }
    }

    Ok(())
}

/// Loads the plugin manifest from a given manifest path.
pub fn load_plugin_manifest<P: AsRef<Path>>(manifest_path: P) -> Result<PluginManifest, String> {
    let manifest_str = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read plugin manifest: {e}"))?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse plugin manifest: {e}"))?;
    // Validate required fields
    if manifest.name.trim().is_empty() {
        return Err("Plugin manifest missing 'name'".to_string());
    }
    if manifest.version.trim().is_empty() {
        return Err("Plugin manifest missing 'version'".to_string());
    }
    if manifest.dynamic_library.trim().is_empty() {
        return Err("Plugin manifest missing 'dynamic_library'".to_string());
    }
    Ok(manifest)
}

/// # Safety
///
/// - `manifest_path` must point to a valid plugin manifest (plugin.json).
/// - The referenced dynamic library must exist and be a valid plugin.
/// - `engine_api` and `world` must be valid for the duration of the plugin.
/// - The caller must ensure all pointer arguments are valid and not aliased elsewhere.
pub unsafe fn load_plugin_with_manifest<P: AsRef<Path>>(
    manifest_path: P,
    engine_api: &mut EngineApi,
    world: *mut c_void,
) -> Result<(), String> {
    let manifest = load_plugin_manifest(&manifest_path)?;
    let manifest_dir = manifest_path
        .as_ref()
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let dylib_path = manifest_dir.join(&manifest.dynamic_library);

    let lib = unsafe { Library::new(&dylib_path) }.map_err(|e| e.to_string())?;
    let vtable: Symbol<*mut *mut PluginVTable> =
        unsafe { lib.get(b"PLUGIN_VTABLE\0") }.map_err(|e| e.to_string())?;
    let plugin_vtable: *mut PluginVTable = unsafe { **vtable };
    if plugin_vtable.is_null() {
        return Err("PLUGIN_VTABLE symbol is null".to_string());
    }
    let vtable_ref: &PluginVTable = unsafe { &*plugin_vtable };

    let init_result = unsafe { (vtable_ref.init)(engine_api as *mut _, world) };
    if init_result != 0 {
        return Err(format!("Plugin init failed with code {}", init_result));
    }

    Ok(())
}

/// Resolves the order in which plugins should be loaded based on their dependencies.
/// Returns a Vec of manifest paths in load order, or an error if there are cycles or missing deps.
pub fn resolve_plugin_load_order(
    manifests: &[(String, PluginManifest)],
) -> Result<Vec<String>, String> {
    // Map plugin name to manifest path
    let name_to_path: HashMap<&str, &String> = manifests
        .iter()
        .map(|(p, m)| (m.name.as_str(), p))
        .collect();

    // 1. Check for missing dependencies
    let mut missing = Vec::new();
    for (_, manifest) in manifests {
        for dep in &manifest.dependencies {
            if !name_to_path.contains_key(dep.as_str()) {
                missing.push(dep.clone());
            }
        }
    }
    if !missing.is_empty() {
        return Err(format!("Missing dependencies: {:?}", missing));
    }

    // 2. Build topo_sort graph
    let mut ts = TopoSort::with_capacity(manifests.len());
    for (_, manifest) in manifests {
        ts.insert(
            manifest.name.as_str(),
            manifest
                .dependencies
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );
    }

    // 3. Try to get a full order, or error if there's a cycle
    let sorted_names = ts
        .try_vec_nodes()
        .map_err(|_| "Cycle detected in plugin dependency graph".to_string())?;

    // 4. Map sorted names back to manifest paths
    let mut order = Vec::new();
    for name in sorted_names {
        if let Some(path) = name_to_path.get(name) {
            order.push((*path).clone());
        }
    }

    Ok(order)
}
