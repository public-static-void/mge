//! Presentation layer for ECS systems.
//!
//! This layer provides a common interface for rendering entities in a world,
//! abstracting away the underlying graphics system.

/// Internal modules
pub mod input;
/// Layouts
pub mod layout;
/// Renderers
pub mod renderer;
/// User interface
pub mod ui;

use crate::map::cell_key::CellKey;
use crate::presentation::renderer::{
    COLOR_DIM_GRAY, COLOR_GRAY, COLOR_VERY_DIM, PresentationRenderer, RenderColor, RenderCommand,
};
use std::collections::HashSet;

/// Presentation system for ECS worlds with schema-driven components.
pub struct PresentationSystem<R: PresentationRenderer> {
    /// The renderer
    pub renderer: R,
}

impl<R: PresentationRenderer> PresentationSystem<R> {
    /// Create a new presentation system
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }

    /// Render the world
    pub fn render_world(&mut self, world: &crate::ecs::world::World) {
        for entity in &world.entities {
            let pos_json = world.get_component(*entity, "Position");
            let renderable_json = world.get_component(*entity, "Renderable");
            if let (Some(pos_json), Some(renderable_json)) = (pos_json, renderable_json) {
                // Extract position (supports Square, Hex, Province)
                let (x, y) = if let Some(pos_obj) = pos_json.get("pos") {
                    if let Some(square) = pos_obj.get("Square") {
                        (
                            square.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            square.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        )
                    } else if let Some(hex) = pos_obj.get("Hex") {
                        (
                            hex.get("q").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            hex.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        )
                    } else if let Some(province) = pos_obj.get("Province") {
                        if let Some(province_id) = province.get("id").and_then(|v| v.as_str()) {
                            if let Some(map) = &world.map {
                                province_centroid(map, province_id).unwrap_or((0, 0))
                            } else {
                                (0, 0)
                            }
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };

                // Extract glyph/color
                if let (Some(glyph), Some(color)) = (
                    renderable_json.get("glyph").and_then(|v| v.as_str()),
                    renderable_json.get("color").and_then(|v| v.as_array()),
                ) && color.len() == 3
                {
                    let r = color[0].as_u64().unwrap_or(255) as u8;
                    let g = color[1].as_u64().unwrap_or(255) as u8;
                    let b = color[2].as_u64().unwrap_or(255) as u8;
                    let cmd = RenderCommand {
                        glyph: glyph.chars().next().unwrap_or('?'),
                        color: RenderColor(r, g, b),
                        pos: (x, y),
                    };
                    self.renderer.queue_draw(cmd);
                }
            }
        }
        self.renderer.present();
    }

    /// Render the map without visibility filtering.
    /// Delegates to [`render_map_with_visibility`](Self::render_map_with_visibility)
    /// with `None` for the visible-cells set.
    pub fn render_map(&mut self, world: &crate::ecs::world::World, viewport: &Viewport) {
        self.render_map_with_visibility(world, viewport, None);
    }

    /// Render the map with optional visibility filtering.
    ///
    /// When `visible_cells` is `Some(set)`:
    /// - Cells NOT in the set are drawn with a dim style.
    /// - Entities in non-visible cells are not drawn.
    ///
    /// When `visible_cells` is `None` everything is drawn normally.
    pub fn render_map_with_visibility(
        &mut self,
        world: &crate::ecs::world::World,
        viewport: &Viewport,
        visible_cells: Option<&HashSet<CellKey>>,
    ) {
        use crate::presentation::layout::{CellLayout, HexLayout, SquareLayout};

        let map = match &world.map {
            Some(m) => m,
            None => return,
        };

        let layout: Box<dyn CellLayout> = match map.topology_type() {
            "square" => Box::new(SquareLayout),
            "hex" => Box::new(HexLayout),
            _ => Box::new(SquareLayout),
        };

        // Draw terrain/background
        for cell in map.all_cells() {
            let (sx, sy) = layout.cell_to_screen(&cell);
            if viewport.contains(sx, sy) {
                let in_visible = visible_cells.map(|vis| vis.contains(&cell)).unwrap_or(true);

                let meta = map.get_cell_metadata(&cell);
                let (glyph, color) = if !in_visible {
                    // Dimmed terrain outside visible set
                    ('.', COLOR_VERY_DIM)
                } else if let Some(meta) = meta {
                    if let Some(terrain) = meta.get("terrain").and_then(|v| v.as_str()) {
                        match terrain {
                            "wall" => ('#', COLOR_GRAY),
                            "floor" => ('.', COLOR_DIM_GRAY),
                            _ => ('.', COLOR_DIM_GRAY),
                        }
                    } else {
                        ('.', COLOR_DIM_GRAY)
                    }
                } else {
                    ('.', COLOR_DIM_GRAY)
                };
                self.renderer.queue_draw(RenderCommand {
                    glyph,
                    color,
                    pos: (sx - viewport.x, sy - viewport.y),
                });
            }
        }

        // Draw entities
        for entity in &world.entities {
            let pos_json = world.get_component(*entity, "Position");
            let renderable_json = world.get_component(*entity, "Renderable");
            if let (Some(pos_json), Some(renderable_json)) = (pos_json, renderable_json) {
                let (x, y, entity_cell) = if let Some(pos_obj) = pos_json.get("pos") {
                    if let Some(square) = pos_obj.get("Square") {
                        let x = square.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let y = square.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let z = square.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        (x, y, Some(CellKey::Square { x, y, z }))
                    } else if let Some(hex) = pos_obj.get("Hex") {
                        let q = hex.get("q").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let r = hex.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let z = hex.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        (q, r, Some(CellKey::Hex { q, r, z }))
                    } else if let Some(province) = pos_obj.get("Province") {
                        if let Some(province_id) = province.get("id").and_then(|v| v.as_str()) {
                            if let Some(map) = &world.map {
                                let (cx, cy) =
                                    province_centroid(map, province_id).unwrap_or((0, 0));
                                (
                                    cx,
                                    cy,
                                    Some(CellKey::Province {
                                        id: province_id.to_string(),
                                    }),
                                )
                            } else {
                                (0, 0, None)
                            }
                        } else {
                            (0, 0, None)
                        }
                    } else {
                        (0, 0, None)
                    }
                } else {
                    (0, 0, None)
                };

                // Skip entities in non-visible cells
                let in_visible = match (&visible_cells, &entity_cell) {
                    (Some(vis), Some(cell)) => vis.contains(cell),
                    (None, _) => true,
                    (Some(_), None) => false,
                };

                if !in_visible {
                    continue;
                }

                if viewport.contains(x, y)
                    && let (Some(glyph), Some(color)) = (
                        renderable_json.get("glyph").and_then(|v| v.as_str()),
                        renderable_json.get("color").and_then(|v| v.as_array()),
                    )
                    && color.len() == 3
                {
                    let r = color[0].as_u64().unwrap_or(255) as u8;
                    let g = color[1].as_u64().unwrap_or(255) as u8;
                    let b = color[2].as_u64().unwrap_or(255) as u8;
                    let cmd = RenderCommand {
                        glyph: glyph.chars().next().unwrap_or('?'),
                        color: RenderColor(r, g, b),
                        pos: (x - viewport.x, y - viewport.y),
                    };
                    self.renderer.queue_draw(cmd);
                }
            }
        }

        self.renderer.present();
    }
}

/// Calculate the centroid of a province for rendering.
/// Returns (x, y) as i32 grid coordinates.
/// This function assumes the map contains provinces as collections of cell positions.
pub fn province_centroid(map: &crate::map::Map, province_id: &str) -> Option<(i32, i32)> {
    if let Some(province_map) = map
        .as_any()
        .downcast_ref::<crate::map::province::ProvinceMap>()
    {
        let cell_ids = province_map.cells.get(province_id)?;
        let mut sum_x = 0i64;
        let mut sum_y = 0i64;
        let mut count = 0i64;
        for cell_id in cell_ids {
            if let Some(meta) = province_map.cell_metadata.get(cell_id)
                && let (Some(x), Some(y)) = (
                    meta.get("x").and_then(|v| v.as_i64()),
                    meta.get("y").and_then(|v| v.as_i64()),
                )
            {
                sum_x += x;
                sum_y += y;
                count += 1;
            }
        }
        if count > 0 {
            Some(((sum_x / count) as i32, (sum_y / count) as i32))
        } else {
            None
        }
    } else {
        None
    }
}

/// A viewport
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// The x position
    pub x: i32,
    /// The y position
    pub y: i32,
    /// The width
    pub width: i32,
    /// The height
    pub height: i32,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a position is in the viewport
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}
