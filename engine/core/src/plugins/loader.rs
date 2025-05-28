use crate::ecs::World;
use crate::plugins::types::{
    EngineApi, LoadedPlugin, PluginManifest, PluginMetadata, PluginVTable, SystemPlugin,
};
use crate::worldgen::{WorldgenPlugin, WorldgenRegistry};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;

/// # Safety
///
/// - `path` must point to a valid dynamic library exposing a compatible plugin vtable.
/// - `engine_api` and `world` must be valid for the duration of the plugin.
/// - The caller must ensure all pointer arguments are valid and not aliased elsewhere.
pub unsafe fn load_plugin<P: AsRef<Path>>(
    path: P,
    engine_api: &mut EngineApi,
    world: *mut c_void,
) -> Result<LoadedPlugin, String> {
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

    let metadata = PluginMetadata {
        manifest: PluginManifest {
            name: "<unknown>".to_string(),
            version: "0.0.0".to_string(),
            description: String::new(),
            authors: vec![],
            dependencies: vec![],
            dynamic_library: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        },
        path: path.as_ref().to_path_buf(),
    };

    Ok(LoadedPlugin::new(lib, plugin_vtable, metadata))
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
) -> Result<LoadedPlugin, String> {
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

        let name = {
            let cstr = unsafe { CStr::from_ptr(worldgen_name_fn()) };
            cstr.to_str().unwrap().to_owned()
        };

        let generate = move |params: &Value| -> Value {
            let params_json = serde_json::to_string(params).unwrap();
            let c_params = CString::new(params_json).unwrap();
            let mut out_ptr: *mut c_char = std::ptr::null_mut();

            let res = unsafe { generate_world_fn(c_params.as_ptr(), &mut out_ptr) };
            if res != 0 || out_ptr.is_null() {
                return Value::Null;
            }
            let result = unsafe { CStr::from_ptr(out_ptr).to_string_lossy().into_owned() };
            unsafe { free_result_json_fn(out_ptr) };
            serde_json::from_str(&result).unwrap_or(Value::Null)
        };

        worldgen_registry.register(WorldgenPlugin::CAbi {
            name,
            generate: Box::new(generate),
        });
    }

    let metadata = PluginMetadata {
        manifest: PluginManifest {
            name: "<unknown>".to_string(),
            version: "0.0.0".to_string(),
            description: String::new(),
            authors: vec![],
            dependencies: vec![],
            dynamic_library: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        },
        path: path.as_ref().to_path_buf(),
    };

    Ok(LoadedPlugin::new(lib, plugin_vtable, metadata))
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
) -> Result<LoadedPlugin, String> {
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

    let metadata = PluginMetadata {
        manifest: PluginManifest {
            name: "<unknown>".to_string(),
            version: "0.0.0".to_string(),
            description: String::new(),
            authors: vec![],
            dependencies: vec![],
            dynamic_library: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        },
        path: path.as_ref().to_path_buf(),
    };

    Ok(LoadedPlugin::new(lib, plugin_vtable, metadata))
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
) -> Result<LoadedPlugin, String> {
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

    let metadata = PluginMetadata {
        manifest,
        path: dylib_path,
    };

    Ok(LoadedPlugin::new(lib, plugin_vtable, metadata))
}
