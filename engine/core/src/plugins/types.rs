use std::os::raw::{c_char, c_float, c_int, c_void};

/// A plugin manifest
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    /// The plugin's name
    pub name: String,
    /// The plugin's version
    pub version: String,
    /// The plugin's description
    #[serde(default)]
    pub description: String,
    /// The plugin's authors
    #[serde(default)]
    pub authors: Vec<String>,
    /// The plugin's dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// The path to the plugin's dynamic library
    pub dynamic_library: String,
}

/// A plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// The plugin's manifest
    pub manifest: PluginManifest,
    /// The plugin's path
    pub path: std::path::PathBuf,
}

/// The engine API
#[repr(C)]
pub struct EngineApi {
    /// Spawns a new entity
    pub spawn_entity: unsafe extern "C" fn(*mut c_void) -> u32,
    /// Sets a component
    pub set_component: unsafe extern "C" fn(*mut c_void, u32, *const c_char, *const c_char) -> i32,
}

/// A system plugin
#[repr(C)]
pub struct SystemPlugin {
    /// The system's name
    pub name: *const c_char,
    /// The system's run function
    pub run: unsafe extern "C" fn(*mut c_void, f32),
}

/// A plugin's vtable
#[repr(C)]
pub struct PluginVTable {
    /// The plugin's init function
    pub init: unsafe extern "C" fn(*mut EngineApi, *mut c_void) -> c_int,
    /// The plugin's shutdown function
    pub shutdown: unsafe extern "C" fn(),
    /// The plugin's update function
    pub update: unsafe extern "C" fn(c_float),
    /// The plugin's worldgen name
    pub worldgen_name: Option<unsafe extern "C" fn() -> *const c_char>,
    /// The plugin's worldgen function
    pub generate_world: Option<unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int>,
    /// The plugin's result parser
    pub free_result_json: Option<unsafe extern "C" fn(*mut c_char)>,
    /// The plugin's system registration function
    pub register_systems: Option<
        unsafe extern "C" fn(
            *mut EngineApi,
            *mut c_void,
            *mut *mut SystemPlugin,
            *mut c_int,
        ) -> c_int,
    >,
    /// The plugin's system unregistration function
    pub free_systems: Option<unsafe extern "C" fn(*mut SystemPlugin, c_int)>,
    /// The plugin's hot-reload function
    pub hot_reload: Option<unsafe extern "C" fn(old_state: *mut c_void) -> *mut c_void>,
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
