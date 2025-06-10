use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap, cell_key::CellKey};
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use engine_core::presentation::{PresentationSystem, Viewport};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn test_map_rendering_renders_terrain_and_entities_in_viewport() {
    // Load config and schemas (assume they are correct and present)
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

    // Register schemas
    let mut registry = ComponentRegistry::new();
    for schema in schemas.values() {
        registry.register_external_schema(schema.clone());
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    // Create a 3x3 square map with terrain metadata
    let mut cells = HashMap::new();
    let mut cell_metadata = HashMap::new();
    for x in 0..3 {
        for y in 0..3 {
            let cell = CellKey::Square { x, y, z: 0 };
            // Add all 4 cardinal neighbors (within map bounds)
            let mut neighbors = HashSet::new();
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x + dx;
                let ny = y + dy;
                if (0..3).contains(&nx) && (0..3).contains(&ny) {
                    neighbors.insert(CellKey::Square { x: nx, y: ny, z: 0 });
                }
            }
            cells.insert(cell.clone(), neighbors);

            let terrain = if x == 1 && y == 1 { "wall" } else { "floor" };
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

    // Spawn an entity at (1, 1) with a Renderable component
    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            json!({ "pos": { "Square": { "x": 1, "y": 1, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(
            entity,
            "Renderable",
            json!({ "glyph": "@", "color": [255, 255, 255] }),
        )
        .unwrap();

    // Set up the renderer and presentation system
    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);

    // Define a viewport covering the whole map
    let viewport = Viewport::new(0, 0, 3, 3);

    // Render the map
    system.render_map(&world, &viewport);

    // Collect draws for easy lookup
    let draws = &system.renderer.draws;
    // There should be 9 terrain draws (one per cell)
    let terrain_draws: Vec<_> = draws
        .iter()
        .filter(|cmd| cmd.glyph == '.' || cmd.glyph == '#')
        .collect();
    assert_eq!(terrain_draws.len(), 9);

    // The wall at (1, 1) should be '#'
    let wall_cmd = draws
        .iter()
        .find(|cmd| cmd.glyph == '#' && cmd.pos == (1, 1))
        .expect("Wall at (1,1) should be drawn");
    assert_eq!(wall_cmd.color, RenderColor(128, 128, 128));

    // There should be one entity draw at (1, 1) with '@'
    let entity_cmd = draws
        .iter()
        .find(|cmd| cmd.glyph == '@' && cmd.pos == (1, 1))
        .expect("Entity at (1,1) should be drawn");
    assert_eq!(entity_cmd.color, RenderColor(255, 255, 255));
}
