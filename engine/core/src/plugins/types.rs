use libloading::Library;
use std::os::raw::{c_char, c_int, c_void};

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub dynamic_library: String,
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub manifest: PluginManifest,
    pub path: std::path::PathBuf,
}

#[repr(C)]
pub struct EngineApi {
    pub spawn_entity: unsafe extern "C" fn(*mut c_void) -> u32,
    pub set_component: unsafe extern "C" fn(*mut c_void, u32, *const c_char, *const c_char) -> i32,
}

#[repr(C)]
pub struct SystemPlugin {
    pub name: *const c_char,
    pub run: unsafe extern "C" fn(*mut c_void, f32),
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
            *mut c_void,
            *mut *mut SystemPlugin,
            *mut c_int,
        ) -> i32,
    >,
    pub free_systems: Option<unsafe extern "C" fn(*mut SystemPlugin, c_int)>,
}

#[derive(Debug)]
pub struct LoadedPlugin {
    _lib: Library, // Must keep alive!
    pub vtable: *const PluginVTable,
    pub metadata: PluginMetadata,
}

impl LoadedPlugin {
    pub fn new(lib: Library, vtable: *const PluginVTable, metadata: PluginMetadata) -> Self {
        Self {
            _lib: lib,
            vtable,
            metadata,
        }
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
