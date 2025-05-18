pub mod dynamic_systems;
use crate::ecs::World;
use crate::worldgen::{WorldgenPlugin, WorldgenRegistry};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::Path;

/// # Safety
///
/// The caller must ensure that `world` is a valid pointer to a `World` object.
/// This function dereferences `world` as a mutable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_spawn_entity(world: *mut c_void) -> c_uint {
    let world = unsafe { &mut *(world as *mut World) };
    world.spawn_entity()
}

/// # Safety
///
/// The caller must ensure that:
/// - `world` is a valid pointer to a `World` object,
/// - `name` and `json_value` are valid null-terminated C strings,
/// - `entity` is a valid entity ID for the given world.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_set_component(
    world: *mut c_void,
    entity: c_uint,
    name: *const c_char,
    json_value: *const c_char,
) -> i32 {
    let world = unsafe { &mut *(world as *mut World) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    let json_value = unsafe { CStr::from_ptr(json_value) }.to_str().unwrap();
    let value: serde_json::Value = serde_json::from_str(json_value).unwrap();
    match world.set_component(entity, name, value) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[repr(C)]
pub struct EngineApi {
    pub spawn_entity: unsafe extern "C" fn(*mut c_void) -> u32,
    pub set_component: unsafe extern "C" fn(*mut c_void, u32, *const c_char, *const c_char) -> i32,
}

#[repr(C)]
pub struct SystemPlugin {
    pub name: *const c_char,
    pub run: unsafe extern "C" fn(*mut std::ffi::c_void, f32),
}

#[repr(C)]
pub struct PluginVTable {
    pub init: unsafe extern "C" fn(*mut EngineApi, *mut c_void) -> i32,
    pub shutdown: unsafe extern "C" fn(),
    pub update: unsafe extern "C" fn(f32),
    pub worldgen_name: unsafe extern "C" fn() -> *const c_char,
    pub generate_world: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> i32,
    pub free_result_json: unsafe extern "C" fn(*mut c_char),
    pub register_systems: Option<
        unsafe extern "C" fn(
            *mut EngineApi,
            *mut std::ffi::c_void,
            *mut *mut SystemPlugin,
            *mut c_int,
        ) -> i32,
    >,
}

pub struct LoadedPlugin {
    _lib: Library, // Must keep alive!
    pub vtable: *const PluginVTable,
}

impl LoadedPlugin {
    pub fn new(lib: Library, vtable: *const PluginVTable) -> Self {
        Self { _lib: lib, vtable }
    }
}

impl SystemPlugin {
    /// Returns the system's name as a string slice.
    ///
    /// # Safety
    /// The caller must ensure that `self.name` is a valid, null-terminated C string.
    pub unsafe fn name_str(&self) -> &str {
        unsafe { std::ffi::CStr::from_ptr(self.name).to_str().unwrap() }
    }
}

/// Loads a plugin from a dynamic library and calls its init function.
///
/// # Safety
/// - The caller must ensure the plugin at `path` is ABI-compatible and exposes a valid vtable.
/// - `engine_api` and `world` must be valid for the duration of the plugin.
/// - This function performs FFI operations and dynamic loading, which may cause undefined behavior if misused.
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
    Ok(LoadedPlugin {
        _lib: lib,
        vtable: plugin_vtable,
    })
}

/// # Safety
/// The caller must ensure the plugin at `path` is ABI-compatible and exposes a valid vtable.
/// `engine_api` and `world` must be valid for the duration of the plugin.
/// This function performs FFI operations and dynamic loading, which may cause undefined behavior if misused.
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

    Ok(LoadedPlugin::new(lib, plugin_vtable))
}

/// Loads a plugin and registers its systems.
///
/// # Safety
/// The caller must ensure the plugin at `path` is ABI-compatible and exposes a valid vtable.
/// `engine_api` and `world` must be valid for the duration of the plugin.
/// This function performs FFI operations and dynamic loading, which may cause undefined behavior if misused.
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
            // TODO: free systems_ptr if allocated dynamically by plugin
        }
    }

    Ok(LoadedPlugin::new(lib, plugin_vtable))
}
