pub mod input;
pub mod layout;
pub mod renderer;

use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};

/// Presentation system for ECS worlds with schema-driven components.
pub struct PresentationSystem<R: PresentationRenderer> {
    pub renderer: R,
}

impl<R: PresentationRenderer> PresentationSystem<R> {
    pub fn new(renderer: R) -> Self {
        Self { renderer }
    }

    pub fn render_world(&mut self, world: &crate::ecs::world::World) {
        for entity in &world.entities {
            let pos_json = world.get_component(*entity, "PositionComponent");
            let renderable_json = world.get_component(*entity, "Renderable");
            if let (Some(pos_json), Some(renderable_json)) = (pos_json, renderable_json) {
                // Extract position (supports Square, Hex, Region)
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
                    } else if let Some(region) = pos_obj.get("Region") {
                        if let Some(region_id) = region.get("id").and_then(|v| v.as_str()) {
                            if let Some(map) = &world.map {
                                region_centroid(map, region_id).unwrap_or((0, 0))
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
                ) {
                    if color.len() == 3 {
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
        }
        self.renderer.present();
    }

    pub fn render_map(&mut self, world: &crate::ecs::world::World, viewport: &Viewport) {
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
                let meta = map.get_cell_metadata(&cell);
                let (glyph, color) = if let Some(meta) = meta {
                    if let Some(terrain) = meta.get("terrain").and_then(|v| v.as_str()) {
                        match terrain {
                            "wall" => ('#', RenderColor(128, 128, 128)),
                            "floor" => ('.', RenderColor(60, 60, 60)),
                            _ => ('.', RenderColor(60, 60, 60)),
                        }
                    } else {
                        ('.', RenderColor(60, 60, 60))
                    }
                } else {
                    ('.', RenderColor(60, 60, 60))
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
            let pos_json = world.get_component(*entity, "PositionComponent");
            let renderable_json = world.get_component(*entity, "Renderable");
            if let (Some(pos_json), Some(renderable_json)) = (pos_json, renderable_json) {
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
                    } else if let Some(region) = pos_obj.get("Region") {
                        if let Some(region_id) = region.get("id").and_then(|v| v.as_str()) {
                            if let Some(map) = &world.map {
                                region_centroid(map, region_id).unwrap_or((0, 0))
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

                if viewport.contains(x, y) {
                    if let (Some(glyph), Some(color)) = (
                        renderable_json.get("glyph").and_then(|v| v.as_str()),
                        renderable_json.get("color").and_then(|v| v.as_array()),
                    ) {
                        if color.len() == 3 {
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
            }
        }

        self.renderer.present();
    }
}

/// Calculate the centroid of a region for rendering.
/// Returns (x, y) as i32 grid coordinates.
/// This function assumes the map contains regions as collections of cell positions.
pub fn region_centroid(map: &crate::map::Map, region_id: &str) -> Option<(i32, i32)> {
    if let Some(region_map) = map.as_any().downcast_ref::<crate::map::region::RegionMap>() {
        let cell_ids = region_map.cells.get(region_id)?;
        let mut sum_x = 0i64;
        let mut sum_y = 0i64;
        let mut count = 0i64;
        for cell_id in cell_ids {
            if let Some(meta) = region_map.cell_metadata.get(cell_id) {
                if let (Some(x), Some(y)) = (
                    meta.get("x").and_then(|v| v.as_i64()),
                    meta.get("y").and_then(|v| v.as_i64()),
                ) {
                    sum_x += x;
                    sum_y += y;
                    count += 1;
                }
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

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Viewport {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}
