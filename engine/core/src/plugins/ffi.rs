use crate::ecs::World;
use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};

/// # Safety
///
/// - `world` must be a valid pointer to a `World`.
/// - Caller must ensure exclusive access to the pointed `World`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_spawn_entity(world: *mut c_void) -> c_uint {
    let world = unsafe { &mut *(world as *mut World) };
    world.spawn_entity()
}

/// # Safety
///
/// - `world` must be a valid pointer to a `World`.
/// - `name` and `json_value` must be valid null-terminated C strings.
/// - `entity` must be a valid entity ID in the given world.
/// - Caller must ensure exclusive access to the pointed `World`.
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
