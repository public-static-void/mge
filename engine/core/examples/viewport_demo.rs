use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::{load_allowed_modes, load_schemas_from_dir_with_modes};
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap, cell_key::CellKey};
use engine_core::presentation::renderer::TerminalRenderer;
use engine_core::presentation::{PresentationSystem, Viewport};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

fn main() {
    // Build a 10x5 map with a wall at (4,2)
    let mut cells = HashMap::new();
    let mut cell_metadata = HashMap::new();
    for x in 0..10 {
        for y in 0..5 {
            let cell = CellKey::Square { x, y, z: 0 };
            let mut neighbors = HashSet::new();
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                if (0..10).contains(&nx) && (0..5).contains(&ny) {
                    neighbors.insert(CellKey::Square { x: nx, y: ny, z: 0 });
                }
            }
            cells.insert(cell.clone(), neighbors);

            let terrain = if x == 4 && y == 2 { "wall" } else { "floor" };
            cell_metadata.insert(cell, json!({ "terrain": terrain }));
        }
    }
    let map = Map {
        topology: Box::new(SquareGridMap {
            cells,
            cell_metadata,
        }),
    };

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

    let width = 10;
    let height = 5;
    let renderer = TerminalRenderer::new(width, height);
    let mut system = PresentationSystem::new(renderer);

    let viewport = Viewport::new(0, 0, width, height);

    system.render_map(&world, &viewport);
}
