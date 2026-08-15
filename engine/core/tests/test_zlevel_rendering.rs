use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use engine_core::presentation::{PresentationSystem, Viewport};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A 3x3 map with two z-levels:
/// - z=0 cells are `floor` (rendered `.`)
/// - z=1 cells are `wall` (rendered `#`)
///
/// Entities: `@` on z=1 at (1,1), `E` on z=0 at (0,0). Terrain per level is
/// deliberately distinct so a test can tell which level was drawn.
fn build_two_level_world() -> (World, u32, u32) {
    let config = GameConfig::load_from_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game.toml"),
    )
    .expect("Failed to load config");
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/schemas");
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &config.allowed_modes)
        .expect("Failed to load schemas");

    let mut registry = ComponentRegistry::new();
    for schema in schemas.values() {
        registry.register_external_schema(schema.clone());
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    world.current_mode = "colony".to_string();

    let mut cells = HashMap::new();
    let mut cell_metadata = HashMap::new();
    for z in 0..2 {
        for x in 0..3 {
            for y in 0..3 {
                let cell = CellKey::Square { x, y, z };
                let mut neighbors = HashSet::new();
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if (0..3).contains(&nx) && (0..3).contains(&ny) {
                        neighbors.insert(CellKey::Square { x: nx, y: ny, z });
                    }
                }
                cells.insert(cell.clone(), neighbors);
                let terrain = if z == 0 { "floor" } else { "wall" };
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

    let entity_z1 = world.spawn_entity();
    world
        .set_component(
            entity_z1,
            "Position",
            json!({ "pos": { "Square": { "x": 1, "y": 1, "z": 1 } } }),
        )
        .unwrap();
    world
        .set_component(
            entity_z1,
            "Renderable",
            json!({ "glyph": "@", "color": [255, 255, 255] }),
        )
        .unwrap();

    let entity_z0 = world.spawn_entity();
    world
        .set_component(
            entity_z0,
            "Position",
            json!({ "pos": { "Square": { "x": 0, "y": 0, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(
            entity_z0,
            "Renderable",
            json!({ "glyph": "E", "color": [255, 255, 255] }),
        )
        .unwrap();

    (world, entity_z1, entity_z0)
}

#[test]
fn test_zfilter_some0_hides_z1_terrain_and_entity() {
    let (world, entity_z1, entity_z0) = build_two_level_world();
    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);
    let viewport = Viewport::new(0, 0, 3, 3);

    system.render_map_with_visibility(&world, &viewport, None, None, Some(0));
    let draws = &system.renderer.draws;

    // z=1 terrain (walls `#`) not drawn at all — not even dimmed
    assert!(
        draws.iter().all(|cmd| cmd.glyph != '#'),
        "z=1 terrain must not be drawn when z filter is Some(0)"
    );
    // z=1 entity not drawn
    assert!(
        draws.iter().all(|cmd| cmd.glyph != '@'),
        "z=1 entity must not be drawn when z filter is Some(0)"
    );
    // z=0 terrain drawn: 9 floors
    let floor_draws = draws.iter().filter(|cmd| cmd.glyph == '.').count();
    assert_eq!(floor_draws, 9, "z=0 terrain should be the only level drawn");
    // z=0 entity drawn
    assert!(
        draws
            .iter()
            .any(|cmd| cmd.glyph == 'E' && cmd.pos == (0, 0)),
        "z=0 entity should be drawn"
    );
    // entity IDs sanity: both entities exist
    assert_ne!(entity_z1, entity_z0);
}

#[test]
fn test_zfilter_some1_draws_z1_only() {
    let (world, _, _) = build_two_level_world();
    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);
    let viewport = Viewport::new(0, 0, 3, 3);

    system.render_map_with_visibility(&world, &viewport, None, None, Some(1));
    let draws = &system.renderer.draws;

    // z=1 terrain (walls `#`) drawn: 9 cells
    let wall_draws = draws.iter().filter(|cmd| cmd.glyph == '#').count();
    assert_eq!(
        wall_draws, 9,
        "z=1 terrain should be drawn when z filter is Some(1)"
    );
    // z=1 entity drawn at its position
    assert!(
        draws
            .iter()
            .any(|cmd| cmd.glyph == '@' && cmd.pos == (1, 1)),
        "z=1 entity should be drawn"
    );
    // z=0 terrain not drawn: no floor `.` glyphs
    assert!(
        draws.iter().all(|cmd| cmd.glyph != '.'),
        "z=0 terrain must not be drawn when z filter is Some(1)"
    );
    // z=0 entity not drawn
    assert!(
        draws.iter().all(|cmd| cmd.glyph != 'E'),
        "z=0 entity must not be drawn when z filter is Some(1)"
    );
}

#[test]
fn test_render_map_all_z_overlay() {
    let (world, _, _) = build_two_level_world();
    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);
    let viewport = Viewport::new(0, 0, 3, 3);

    // render_map delegates with z: None — all-z overlay (legacy behavior)
    system.render_map(&world, &viewport);
    let draws = &system.renderer.draws;

    let floor_draws = draws.iter().filter(|cmd| cmd.glyph == '.').count();
    let wall_draws = draws.iter().filter(|cmd| cmd.glyph == '#').count();
    assert_eq!(floor_draws, 9, "z=0 terrain drawn in all-z overlay");
    assert_eq!(wall_draws, 9, "z=1 terrain drawn in all-z overlay");
    assert!(
        draws.iter().any(|cmd| cmd.glyph == '@'),
        "z=1 entity drawn in all-z overlay"
    );
    assert!(
        draws.iter().any(|cmd| cmd.glyph == 'E'),
        "z=0 entity drawn in all-z overlay"
    );
}

#[test]
fn test_zfilter_with_visibility_never_dims_other_z() {
    let (world, _, _) = build_two_level_world();
    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);
    let viewport = Viewport::new(0, 0, 3, 3);

    // Only one z=1 cell is visible; the z=0 level is entirely outside the set.
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 1 });

    system.render_map_with_visibility(&world, &viewport, Some(&visible), None, Some(1));
    let draws = &system.renderer.draws;

    // Visible z=1 cell drawn normally as a wall
    assert!(
        draws.iter().any(|cmd| cmd.pos == (0, 0)
            && cmd.glyph == '#'
            && cmd.color == RenderColor(128, 128, 128)),
        "visible z=1 cell should be drawn normally"
    );
    // Same-z visibility logic unchanged: the other 8 z=1 cells are dimmed
    let dimmed = draws
        .iter()
        .filter(|cmd| cmd.color == RenderColor(25, 25, 25))
        .count();
    assert_eq!(dimmed, 8, "same-z non-visible cells should be dimmed");
    // Non-selected z cells never render dimmed/unexplored: exactly the 8 z=1
    // dimmed draws exist — if z=0 leaked through it would add 9 more.
    let dot_draws = draws.iter().filter(|cmd| cmd.glyph == '.').count();
    assert_eq!(
        dot_draws, 8,
        "z=0 cells must be filtered before visibility checks"
    );
    // z=0 entity not drawn
    assert!(
        draws.iter().all(|cmd| cmd.glyph != 'E'),
        "z=0 entity must not be drawn when z filter is active"
    );
}
