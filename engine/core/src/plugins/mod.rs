use crate::scripting::World;
use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};
use std::path::Path;

/// Spawns a new entity in the ECS world.
///
/// # Safety
/// - `world` must be a valid, non-null pointer to a `World` instance.
/// - The caller must ensure that no data races or aliasing violations occur.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_spawn_entity(world: *mut c_void) -> c_uint {
    let world = unsafe { &mut *(world as *mut World) };
    world.spawn_entity()
}

/// Sets a component on an entity in the ECS world.
///
/// # Safety
/// - `world` must be a valid, non-null pointer to a `World` instance.
/// - `name` and `json_value` must be valid, null-terminated C strings.
/// - The caller must ensure that no data races or aliasing violations occur.
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
    pub spawn_entity: unsafe extern "C" fn(*mut std::os::raw::c_void) -> u32,
    pub set_component: unsafe extern "C" fn(
        *mut std::os::raw::c_void,
        u32,
        *const std::os::raw::c_char,
        *const std::os::raw::c_char,
    ) -> i32,
    // Add more function pointers as ABI grows
}

#[repr(C)]
pub struct PluginVTable {
    pub init: unsafe extern "C" fn(*const EngineApi, *mut c_void) -> i32,
    pub shutdown: unsafe extern "C" fn(),
    pub update: unsafe extern "C" fn(f32),
}

pub struct LoadedPlugin {
    _lib: Library, // Must keep alive!
    pub vtable: *const PluginVTable,
}

/// Loads a plugin from a dynamic library and calls its init function.
///
/// # Safety
/// - The caller must ensure the plugin at `path` is ABI-compatible and exposes a valid vtable.
/// - `engine_api` and `world` must be valid for the duration of the plugin.
/// - This function performs FFI operations and dynamic loading, which may cause undefined behavior if misused.
pub unsafe fn load_plugin<P: AsRef<Path>>(
    path: P,
    engine_api: &EngineApi,
    world: *mut c_void,
) -> Result<LoadedPlugin, String> {
    // Library::new is unsafe and must be wrapped
    let lib = unsafe { Library::new(path.as_ref()) }.map_err(|e| e.to_string())?;
    // Library::get is unsafe and must be wrapped
    let vtable: Symbol<*const PluginVTable> =
        unsafe { lib.get(b"PLUGIN_VTABLE\0") }.map_err(|e| e.to_string())?;
    let plugin_vtable = *vtable;
    let vtable_ref = unsafe { &*plugin_vtable };
    // Calling a function pointer from FFI is unsafe
    let init_result = unsafe { (vtable_ref.init)(engine_api as *const _, world) };
    if init_result != 0 {
        return Err(format!("Plugin init failed with code {}", init_result));
    }
    Ok(LoadedPlugin {
        _lib: lib,
        vtable: plugin_vtable,
    })
}
