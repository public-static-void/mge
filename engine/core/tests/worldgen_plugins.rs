use engine_core::ecs::{ComponentRegistry, World};
use engine_core::plugins::{EngineApi, load_plugin_and_register_worldgen};
use engine_core::worldgen::WorldgenRegistry;
use serde_json::json;
use std::path::Path;
use std::sync::{Arc, Mutex};

unsafe extern "C" fn dummy_spawn_entity(_world: *mut std::os::raw::c_void) -> u32 {
    0
}

unsafe extern "C" fn dummy_set_component(
    _world: *mut std::os::raw::c_void,
    _entity: u32,
    _name: *const std::os::raw::c_char,
    _json_value: *const std::os::raw::c_char,
) -> i32 {
    0
}

fn load_plugin_by_name(
    registry: &mut WorldgenRegistry,
    engine_api: &mut EngineApi,
    world: *mut std::os::raw::c_void,
    plugin_dir: &str,
    plugin_name: &str,
) {
    let plugin_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join(plugin_dir)
        .join(format!("lib{}_plugin.so", plugin_name));

    let result =
        unsafe { load_plugin_and_register_worldgen(plugin_path, engine_api, world, registry) };
    assert!(
        result.is_ok(),
        "Plugin {} should load successfully",
        plugin_name
    );
}

/// Test loading and invoking the simple_square C plugin with detailed data validation.
#[test]
fn test_simple_square_plugin() {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: dummy_spawn_entity,
        set_component: dummy_set_component,
    };
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut dummy_world = World::new(component_registry);
    let world: *mut std::os::raw::c_void = &mut dummy_world as *mut _ as *mut _;

    load_plugin_by_name(
        &mut registry,
        &mut engine_api,
        world,
        "simple_square_plugin",
        "simple_square",
    );
    assert!(registry.list_names().contains(&"simple_square".to_string()));

    let params = json!({
        "width": 2,
        "height": 2,
        "z_levels": 1,
        "chunk_x": 0,
        "chunk_y": 0,
    });

    let map = registry
        .invoke("simple_square", &params)
        .expect("simple_square plugin invocation");
    assert_eq!(map.get("topology").unwrap(), "square");

    let cells = map.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 2 * 2 * 1);

    for cell in cells {
        let x = cell.get("x").unwrap().as_i64().unwrap();
        let y = cell.get("y").unwrap().as_i64().unwrap();
        let z = cell.get("z").unwrap().as_i64().unwrap();
        assert!(x >= 0 && x < 2);
        assert!(y >= 0 && y < 2);
        assert_eq!(z, 0);

        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();
        // Each cell should have between 2 and 4 neighbors in a 2x2 with 4-way adjacency
        assert!((2..=4).contains(&neighbors.len()));

        for neighbor in neighbors {
            let nx = neighbor.get("x").unwrap().as_i64().unwrap();
            let ny = neighbor.get("y").unwrap().as_i64().unwrap();
            let nz = neighbor.get("z").unwrap().as_i64().unwrap();
            assert_eq!(nz, 0);
            // Neighbor must be adjacent and within bounds
            assert!(nx >= 0 && nx < 2);
            assert!(ny >= 0 && ny < 2);
            let dx = (nx - x).abs();
            let dy = (ny - y).abs();
            assert!(dx + dy == 1);
        }

        // If biome/terrain are present, check they are valid strings
        if let Some(biome) = cell.get("biome") {
            assert!(biome.is_string());
        }
        if let Some(terrain) = cell.get("terrain") {
            assert!(terrain.is_string());
        }
    }
}

/// Test loading and invoking the simple_hex C plugin with detailed data validation.
#[test]
fn test_simple_hex_plugin() {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: dummy_spawn_entity,
        set_component: dummy_set_component,
    };
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut dummy_world = World::new(component_registry);
    let world: *mut std::os::raw::c_void = &mut dummy_world as *mut _ as *mut _;

    load_plugin_by_name(
        &mut registry,
        &mut engine_api,
        world,
        "simple_hex_plugin",
        "simple_hex",
    );
    assert!(registry.list_names().contains(&"simple_hex".to_string()));

    let params = json!({
        "width": 3,
        "height": 3,
        "z_levels": 1,
        "chunk_q": 0,
        "chunk_r": 0,
    });

    let map = registry
        .invoke("simple_hex", &params)
        .expect("simple_hex plugin invocation");
    assert_eq!(map.get("topology").unwrap(), "hex");

    let cells = map.get("cells").unwrap().as_array().unwrap();
    assert_eq!(cells.len(), 3 * 3 * 1);

    for cell in cells {
        let q = cell.get("q").unwrap().as_i64().unwrap();
        let r = cell.get("r").unwrap().as_i64().unwrap();
        let z = cell.get("z").unwrap().as_i64().unwrap();
        assert!(q >= 0 && q < 3);
        assert!(r >= 0 && r < 3);
        assert_eq!(z, 0);

        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();
        // Hex cell max 6 neighbors
        assert!(neighbors.len() <= 6);

        for neighbor in neighbors {
            let nq = neighbor.get("q").unwrap().as_i64().unwrap();
            let nr = neighbor.get("r").unwrap().as_i64().unwrap();
            let nz = neighbor.get("z").unwrap().as_i64().unwrap();
            assert_eq!(nz, 0);
            assert!(nq >= 0 && nq < 3);
            assert!(nr >= 0 && nr < 3);
            // neighbors should be adjacent - neighbor axial distance = 1 (approx)
            let dq = (nq - q).abs();
            let dr = (nr - r).abs();
            assert!((dq <= 1) && (dr <= 1));
        }

        // Validate biome and terrain if present
        if let Some(biome) = cell.get("biome") {
            assert!(biome.is_string());
        }
        if let Some(terrain) = cell.get("terrain") {
            assert!(terrain.is_string());
        }
    }
}

/// Test loading and invoking the province worldgen C plugin with detailed data validation.
#[test]
fn test_province_plugin() {
    let mut registry = WorldgenRegistry::new();

    let mut engine_api = EngineApi {
        spawn_entity: dummy_spawn_entity,
        set_component: dummy_set_component,
    };
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut dummy_world = World::new(component_registry);
    let world: *mut std::os::raw::c_void = &mut dummy_world as *mut _ as *mut _;

    load_plugin_by_name(
        &mut registry,
        &mut engine_api,
        world,
        "simple_province_plugin",
        "simple_province",
    );
    assert!(
        registry
            .list_names()
            .contains(&"simple_province".to_string())
    );

    let params = json!({});

    let map = registry
        .invoke("simple_province", &params)
        .expect("simple_province plugin invocation");
    assert_eq!(map.get("topology").unwrap(), "province");

    let cells = map.get("cells").unwrap().as_array().unwrap();

    // Expect exactly 3 provinces based on plugin code
    assert_eq!(cells.len(), 3);

    // Validate each cell's id and neighbors explicitly
    for cell in cells {
        let id = cell.get("id").unwrap().as_str().unwrap();
        let neighbors = cell.get("neighbors").unwrap().as_array().unwrap();

        match id {
            "A" => {
                assert_eq!(neighbors.len(), 2);
                let mut neighbor_ids: Vec<_> =
                    neighbors.iter().map(|n| n.as_str().unwrap()).collect();
                neighbor_ids.sort();
                assert_eq!(neighbor_ids, ["B", "C"]);
            }
            "B" => {
                assert_eq!(neighbors.len(), 1);
                assert_eq!(neighbors[0].as_str().unwrap(), "A");
            }
            "C" => {
                assert_eq!(neighbors.len(), 1);
                assert_eq!(neighbors[0].as_str().unwrap(), "A");
            }
            _ => panic!("Unexpected province id {}", id),
        }
    }
}
