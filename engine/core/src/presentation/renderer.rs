//! Renderer abstraction for the presentation layer.

use crate::map::cell_key::CellKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderColor(pub u8, pub u8, pub u8);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderCommand {
    pub glyph: char,
    pub color: RenderColor,
    pub pos: (i32, i32),
}

/// Trait for a minimal, modular presentation renderer.
pub trait PresentationRenderer {
    /// Queue a draw command (glyph at position with color).
    fn queue_draw(&mut self, cmd: RenderCommand);

    /// Queue a draw command for a specific cell.
    fn queue_draw_cell(&mut self, pos: (i32, i32), cell: &CellKey);

    /// Present all queued draw commands to the screen.
    fn present(&mut self);

    /// Clear the frame (optional, default: no-op).
    fn clear(&mut self) {
        // No-op by default.
    }
}

/// Example: a headless/test renderer that records draw calls.
pub struct TestRenderer {
    pub draws: Vec<RenderCommand>,
    pub cells: Vec<(i32, i32, CellKey)>,
}

impl TestRenderer {
    pub fn new() -> Self {
        Self {
            draws: Vec::new(),
            cells: Vec::new(),
        }
    }
}

impl PresentationRenderer for TestRenderer {
    fn queue_draw(&mut self, cmd: RenderCommand) {
        self.draws.push(cmd);
    }
    fn queue_draw_cell(&mut self, pos: (i32, i32), cell: &CellKey) {
        self.cells.push((pos.0, pos.1, cell.clone()));
    }
    fn present(&mut self) {
        // No-op for tests.
    }
}

impl Default for TestRenderer {
    fn default() -> Self {
        Self::new()
    }
}
