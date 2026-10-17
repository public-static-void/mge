//! Native planet-biome worldgen plugin.
//!
//! Connects the worldgen C ABI hook to the in-core planet generator, so a
//! planet map can be emitted through a native plugin entry rather than the
//! builtin. Emission delegates to [`engine_core::systems::planet_biomes`] so
//! the plugin output stays byte-identical to the authoritative in-core
//! generator. The plugin owns none of the climate model; it is a thin bridge
//! from the C ABI JSON in/out contract to that generator.

use engine_core::plugins::{EngineApi, PLUGIN_ABI_VERSION, PluginVTable};
use engine_core::systems::planet_biomes::PlanetBiomeGenerator;
use engine_core::worldgen::MapgenAlgorithm;
use libc::{c_char, c_float, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;

/// Registry name this plugin registers under. Deliberately distinct from the
/// builtin `planet_biomes`: the loader registers with duplicate-rejection, so a
/// shared name would fail on any builtin-populated registry.
const WORLDGEN_NAME: &str = "planet_biomes_native\0";

/// Returns this plugin's registry name as a NUL-terminated static C string.
unsafe extern "C" fn worldgen_name() -> *const c_char {
    WORLDGEN_NAME.as_ptr() as *const c_char
}

/// Emits a planet map for `params_json` into `out`.
///
/// Any parse or generation failure returns nonzero without writing `out`, so
/// the loader maps the invocation to a null value and the shared pipeline
/// rejects it at schema validation instead of advancing a panic.
unsafe extern "C" fn generate_world(params_json: *const c_char, out: *mut *mut c_char) -> c_int {
    if params_json.is_null() || out.is_null() {
        return 1;
    }
    let params_str = match unsafe { CStr::from_ptr(params_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return 1,
    };
    let params: serde_json::Value = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(_) => return 1,
    };
    let map = match PlanetBiomeGenerator.generate(&params) {
        Ok(m) => m,
        Err(_) => return 1,
    };
    let json = match serde_json::to_string(&map) {
        Ok(j) => j,
        Err(_) => return 1,
    };
    let json_c = match CString::new(json) {
        Ok(c) => c,
        Err(_) => return 1,
    };
    unsafe { *out = json_c.into_raw() };
    0
}

/// Reclaims the string produced by [`generate_world`] via [`CString::into_raw`].
unsafe extern "C" fn free_result_json(result_json: *mut c_char) {
    if !result_json.is_null() {
        drop(unsafe { CString::from_raw(result_json) });
    }
}

unsafe extern "C" fn init(_api: *mut EngineApi, _world: *mut c_void) -> c_int {
    0
}

unsafe extern "C" fn shutdown() {}

unsafe extern "C" fn update(_delta_time: c_float) {}

/// Plugin vtable, exported for the engine loader. Populated by the `#[ctor]`
/// below; the loader reads the pointer through this symbol after `Library::new`.
#[unsafe(no_mangle)]
pub static mut PLUGIN_VTABLE: *mut PluginVTable = ptr::null_mut();

#[ctor::ctor]
fn init_vtable() {
    static mut VTABLE: PluginVTable = PluginVTable {
        abi_version: PLUGIN_ABI_VERSION,
        init,
        shutdown,
        update,
        worldgen_name: Some(worldgen_name),
        generate_world: Some(generate_world),
        free_result_json: Some(free_result_json),
        register_systems: None,
        free_systems: None,
        hot_reload: None,
    };
    unsafe {
        PLUGIN_VTABLE = ptr::addr_of_mut!(VTABLE);
    }
}
