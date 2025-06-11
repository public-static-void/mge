//! Interactive camera movement demo. Use WASD to move camera, q to quit.

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap, cell_key::CellKey};
use engine_core::presentation::renderer::TerminalRenderer;
use engine_core::presentation::{PresentationSystem, Viewport};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

fn main() {
    // Load schemas with mode validation
    let schema_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let allowed_modes = load_allowed_modes().expect("Failed to load allowed modes");
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &allowed_modes)
        .expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for schema in schemas.values() {
        registry.register_external_schema(schema.clone());
    }

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    println!("Current mode: {}", world.current_mode);
    println!(
        "Registered schemas: {:?}",
        registry.lock().unwrap().all_component_names()
    );
    if let Some(schema) = registry.lock().unwrap().get_schema_by_name("Camera") {
        println!("Camera schema loaded: {:?}", schema.modes);
    }

    // Build a 20x10 map with border and sprinkled walls
    let map_width = 20;
    let map_height = 10;
    let mut cells = HashMap::new();
    let mut cell_metadata = HashMap::new();
    for x in 0..map_width {
        for y in 0..map_height {
            let cell = CellKey::Square { x, y, z: 0 };
            let mut neighbors = HashSet::new();
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                if (0..map_width).contains(&nx) && (0..map_height).contains(&ny) {
                    neighbors.insert(CellKey::Square { x: nx, y: ny, z: 0 });
                }
            }
            cells.insert(cell.clone(), neighbors);

            // Add more walls for visual clarity
            let terrain = if x == 0 || y == 0 || x == map_width - 1 || y == map_height - 1 {
                // border walls
                "wall"
            } else if (x + y) % 7 == 0 {
                // sprinkle some random walls
                "wall"
            } else {
                "floor"
            };
            cell_metadata.insert(cell, json!({ "terrain": terrain }));
        }
    }
    let map = Map {
        topology: Box::new(SquareGridMap {
            cells,
            cell_metadata,
        }),
    };
    world.map = Some(map);

    // Spawn an entity at (4, 2)
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            json!({ "pos": { "Square": { "x": 4, "y": 2, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(
            entity,
            "Renderable",
            json!({ "glyph": "@", "color": [255, 255, 255] }),
        )
        .unwrap();

    // Spawn camera at (2, 2)
    let camera = world.spawn_entity();
    world
        .set_component(
            camera,
            "Position",
            json!({ "pos": { "Square": { "x": 2, "y": 2, "z": 0 } } }),
        )
        .unwrap();
    world.set_component(camera, "Camera", json!({})).unwrap();

    let width = 10;
    let height = 5;
    let renderer = TerminalRenderer::new(width, height);
    let mut system = PresentationSystem::new(renderer);

    loop {
        // Get camera position
        let cam_pos = world.get_component(camera, "Position").unwrap();
        let x = cam_pos["pos"]["Square"]["x"].as_i64().unwrap();
        let y = cam_pos["pos"]["Square"]["y"].as_i64().unwrap();

        // Center viewport on camera (with clamping to map bounds)
        let viewport_x = (x as i32 - width / 2).clamp(0, map_width - width);
        let viewport_y = (y as i32 - height / 2).clamp(0, map_height - height);
        let viewport = Viewport::new(viewport_x, viewport_y, width, height);

        system.render_map(&world, &viewport);

        println!("Camera position: ({}, {})", x, y);
        println!("Use WASD to move camera, q to quit:");
        let mut buf = [0; 1];
        io::stdin().read_exact(&mut buf).unwrap();
        let ch = buf[0] as char;
        if ch == 'q' {
            break;
        }

        // --- Clamp camera movement to map bounds only ---
        let (dx, dy) = match ch {
            'w' => (0, -1),
            's' => (0, 1),
            'a' => (-1, 0),
            'd' => (1, 0),
            _ => (0, 0),
        };

        let new_x = (x + dx).clamp(0, (map_width - 1) as i64);
        let new_y = (y + dy).clamp(0, (map_height - 1) as i64);

        if new_x != x || new_y != y {
            let mut pos = cam_pos.clone();
            pos["pos"]["Square"]["x"] = json!(new_x);
            pos["pos"]["Square"]["y"] = json!(new_y);
            world.set_component(camera, "Position", pos).unwrap();
        }
    }
}
