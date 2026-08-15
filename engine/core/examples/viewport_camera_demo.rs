//! Interactive camera movement demo with z-level switching.
//!
//! Demonstrates the presentation-layer z-stacking chain: the camera entity
//! carries a z-level (`pos.Square.z`), the viewport is built for that z, and
//! the renderer filters terrain and entities by z (`render_map_with_visibility`
//! with `Some(z)`). Use WASD to move the camera, `[` / `]` to switch
//! z-levels, q to quit.

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

/// Number of z-levels in the demo map (z=0 and z=1).
const MAP_Z_LEVELS: i64 = 2;
/// Highest z-level index.
const MAP_Z_MAX: i64 = MAP_Z_LEVELS - 1;

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

    // Build a 2-level 20x10 map. Each level is its own z-plane (no cross-z
    // neighbor edges) with a distinct terrain pattern so the active level is
    // recognizable: z=0 uses border walls + sprinkled walls on floor; z=1 uses
    // a checkerboard.
    let map_width = 20;
    let map_height = 10;
    let mut cells = HashMap::new();
    let mut cell_metadata = HashMap::new();
    for z in 0..MAP_Z_LEVELS as i32 {
        for x in 0..map_width {
            for y in 0..map_height {
                let cell = CellKey::Square { x, y, z };
                let mut neighbors = HashSet::new();
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if (0..map_width).contains(&nx) && (0..map_height).contains(&ny) {
                        neighbors.insert(CellKey::Square { x: nx, y: ny, z });
                    }
                }
                cells.insert(cell.clone(), neighbors);

                let terrain = if z == 0 {
                    // Level 0: border walls + sprinkled interior walls
                    if x == 0
                        || y == 0
                        || x == map_width - 1
                        || y == map_height - 1
                        || (x + y) % 7 == 0
                    {
                        "wall"
                    } else {
                        "floor"
                    }
                } else {
                    // Level 1: checkerboard — visually distinct from level 0
                    if (x + y) % 2 == 0 { "wall" } else { "floor" }
                };
                cell_metadata.insert(cell, json!({ "terrain": terrain }));
            }
        }
    }
    let map = Map {
        topology: Box::new(SquareGridMap {
            cells,
            cell_metadata,
        }),
    };
    world.map = Some(map);

    // Spawn the player entity on z=0
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

    // Spawn a distinct entity on z=1 — only drawn while the camera is on z=1
    let entity_z1 = world.spawn_entity();
    world
        .set_component(
            entity_z1,
            "Position",
            json!({ "pos": { "Square": { "x": 4, "y": 2, "z": 1 } } }),
        )
        .unwrap();
    world
        .set_component(
            entity_z1,
            "Renderable",
            json!({ "glyph": "D", "color": [0, 255, 0] }),
        )
        .unwrap();

    // Spawn camera at (2, 2) on z=0
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
        // Get camera position — z read from pos.Square.z (the single source
        // the camera z is written to)
        let cam_pos = world.get_component(camera, "Position").unwrap();
        let x = cam_pos["pos"]["Square"]["x"].as_i64().unwrap();
        let y = cam_pos["pos"]["Square"]["y"].as_i64().unwrap();
        let z = cam_pos["pos"]["Square"]["z"].as_i64().unwrap_or(0);

        // Center viewport on camera (with clamping to map bounds) on the
        // camera's z-level
        let viewport_x = (x as i32 - width / 2).clamp(0, map_width - width);
        let viewport_y = (y as i32 - height / 2).clamp(0, map_height - height);
        let viewport = Viewport::with_z(viewport_x, viewport_y, width, height, z as i32);

        // Render only the camera's z-level. Cross-z cells and entities are
        // filtered before visibility checks, so other levels never appear
        // dimmed or unexplored.
        system.render_map_with_visibility(&world, &viewport, None, None, Some(z as i32));

        println!("Camera position: ({x}, {y}, z={z})");
        println!("Use WASD to move camera, [ / ] to switch z-level, q to quit:");
        let mut buf = [0; 1];
        io::stdin().read_exact(&mut buf).unwrap();
        let ch = buf[0] as char;
        if ch == 'q' {
            break;
        }

        // --- Clamp camera movement to map bounds; z-switch between levels ---
        let (dx, dy) = match ch {
            'w' => (0, -1),
            's' => (0, 1),
            'a' => (-1, 0),
            'd' => (1, 0),
            _ => (0, 0),
        };
        let dz = match ch {
            '[' => -1,
            ']' => 1,
            _ => 0,
        };

        let new_x = (x + dx).clamp(0, (map_width - 1) as i64);
        let new_y = (y + dy).clamp(0, (map_height - 1) as i64);
        let new_z = (z + dz).clamp(0, MAP_Z_MAX);

        if new_x != x || new_y != y || new_z != z {
            // move_entity_3d shifts the existing Position component in place
            // (preserving other fields); here it moves the camera's level when
            // [ or ] was pressed.
            world.move_entity_3d(
                camera,
                (new_x - x) as f32,
                (new_y - y) as f32,
                (new_z - z) as f32,
            );
        }
    }
}
