#[path = "helpers/worldgen.rs"]
mod worldgen_helper;
use worldgen_helper::setup_registry_with_c_plugin;

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::map::CellKey;
use serde_json::json;
use std::sync::{Arc, Mutex};

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
