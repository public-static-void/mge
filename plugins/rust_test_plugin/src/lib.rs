use libc::{c_char, c_float, c_int, c_uint, c_void};
use std::ffi::CString;
use std::ptr;
use std::ptr::addr_of_mut;

#[repr(C)]
pub struct EngineApi {
    pub spawn_entity: unsafe extern "C" fn(*mut c_void) -> c_uint,
    pub set_component:
        unsafe extern "C" fn(*mut c_void, c_uint, *const c_char, *const c_char) -> c_int,
}

pub type WorldPtr = *mut c_void;
pub type SystemRunFn = unsafe extern "C" fn(WorldPtr, c_float);

#[repr(C)]
pub struct SystemPlugin {
    pub name: *const c_char,
    pub run: SystemRunFn,
}

#[repr(C)]
pub struct PluginVTable {
    pub init: unsafe extern "C" fn(*mut EngineApi, *mut c_void) -> c_int,
    pub shutdown: unsafe extern "C" fn(),
    pub update: unsafe extern "C" fn(c_float),
    pub worldgen_name: Option<unsafe extern "C" fn() -> *const c_char>,
    pub generate_world: Option<unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int>,
    pub free_result_json: Option<unsafe extern "C" fn(*mut c_char)>,
    pub register_systems: Option<
        unsafe extern "C" fn(
            *mut EngineApi,
            *mut c_void,
            *mut *mut SystemPlugin,
            *mut c_int,
        ) -> c_int,
    >,
    pub free_systems: Option<unsafe extern "C" fn(*mut SystemPlugin, c_int)>,
    pub hot_reload: Option<unsafe extern "C" fn(old_state: *mut c_void) -> *mut c_void>,
}

// --- System implementation ---
unsafe extern "C" fn hello_system(_world: WorldPtr, _delta_time: c_float) {
    println!("[RUST PLUGIN] Hello from Rust system!");
}

static SYSTEM_NAME: &str = "rust_hello_system\0";
static mut SYSTEM_PLUGINS: [SystemPlugin; 1] = [SystemPlugin {
    name: SYSTEM_NAME.as_ptr() as *const c_char,
    run: hello_system,
}];

unsafe extern "C" fn register_systems(
    _api: *mut EngineApi,
    _world: *mut c_void,
    systems: *mut *mut SystemPlugin,
    count: *mut c_int,
) -> c_int {
    *systems = addr_of_mut!(SYSTEM_PLUGINS[0]);
    *count = 1;
    0
}

unsafe extern "C" fn init(api: *mut EngineApi, world: *mut c_void) -> c_int {
    let api = &*api;
    let entity = (api.spawn_entity)(world);
    let pos_json = CString::new(r#"{"x": 10.0, "y": 42.0}"#).unwrap();
    let comp_name = CString::new("Position").unwrap();
    let result = (api.set_component)(world, entity, comp_name.as_ptr(), pos_json.as_ptr());
    println!("[RUST PLUGIN] Initialized: spawned entity {entity} with Position");
    result
}

unsafe extern "C" fn shutdown() {
    println!("[RUST PLUGIN] Shutdown");
}

unsafe extern "C" fn update(dt: c_float) {
    println!("[RUST PLUGIN] Update called with dt={dt}");
}

unsafe extern "C" fn hot_reload(old_state: *mut c_void) -> *mut c_void {
    old_state
}

// --- VTable setup ---
#[no_mangle]
pub static mut PLUGIN_VTABLE: *mut PluginVTable = ptr::null_mut();

#[ctor::ctor]
fn init_vtable() {
    static mut VTABLE: PluginVTable = PluginVTable {
        init,
        shutdown,
        update,
        worldgen_name: None,
        generate_world: None,
        free_result_json: None,
        register_systems: Some(register_systems),
        free_systems: None,
        hot_reload: Some(hot_reload),
    };
    unsafe {
        PLUGIN_VTABLE = std::ptr::addr_of_mut!(VTABLE);
    }
}
