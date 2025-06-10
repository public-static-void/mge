use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::map::CellKey;
use engine_core::plugins::loader::load_plugin_and_register_worldgen;
use engine_core::plugins::types::EngineApi;
use engine_core::worldgen::WorldgenRegistry;
use serde_json::json;
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Minimal C ABI-compatible spawn_entity function for testing
unsafe extern "C" fn test_spawn_entity(_world: *mut c_void) -> u32 {
    0
}

/// Minimal C ABI-compatible set_component function for testing
unsafe extern "C" fn test_set_component(
    _world: *mut c_void,
    _entity: u32,
    _name: *const c_char,
    _json_value: *const c_char,
) -> i32 {
    0
}

fn setup_registry_with_c_plugin() -> WorldgenRegistry {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: test_spawn_entity,
        set_component: test_set_component,
    };
    let world_ptr = std::ptr::null_mut();

    // Find workspace root by going up until we see a "plugins" directory
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("plugins").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing 'plugins' directory");
        }
    }
    let plugin_path = dir.join("plugins/simple_square_plugin/libsimple_square_plugin.so");

    println!("Loading plugin from: {:?}", plugin_path);
    println!("Plugin path exists: {}", plugin_path.exists());
    println!("CWD: {:?}", std::env::current_dir().unwrap());

    assert!(
        plugin_path.exists(),
        "Plugin .so not found at {:?}. CWD: {:?}",
        plugin_path,
        std::env::current_dir().unwrap()
    );

    unsafe {
        load_plugin_and_register_worldgen(
            plugin_path.to_str().unwrap(),
            &mut engine_api,
            world_ptr,
            &mut registry,
        )
        .expect("Failed to load C plugin");
    }

    registry
}

#[test]
fn test_apply_generated_map_to_world() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    // Register and invoke C worldgen plugin
    let worldgen_registry = setup_registry_with_c_plugin();
    let params = json!({ "width": 4, "height": 4, "z_levels": 1, "seed": 42 });
    let map_json = worldgen_registry.invoke("simple_square", &params).unwrap();

    // Apply the map to the world
    world.apply_generated_map(&map_json).unwrap();

    // Topology-agnostic assertions
    let map = world.get_map().unwrap();
    assert_eq!(map.topology_type(), "square");
    assert_eq!(map.all_cells().len(), 16); // 4x4x1
    assert!(map.contains(&CellKey::Square { x: 0, y: 0, z: 0 }));
    assert!(map.contains(&CellKey::Square { x: 3, y: 3, z: 0 }));
}
