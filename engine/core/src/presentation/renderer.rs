//! Renderer abstraction for the presentation layer.

use crate::map::cell_key::CellKey;
use serde::{Deserialize, Serialize};

/// A color in RGB space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderColor(pub u8, pub u8, pub u8);

/// A command to draw a glyph at a position with a color
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderCommand {
    /// Glyph to draw
    pub glyph: char,
    /// Color
    pub color: RenderColor,
    /// Position
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
    /// Draw commands
    pub draws: Vec<RenderCommand>,
    /// Cells to draw
    pub cells: Vec<(i32, i32, CellKey)>,
}

impl TestRenderer {
    /// Create a new test renderer
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

/// Terminal renderer
pub struct TerminalRenderer {
    /// Terminal width
    pub width: i32,
    /// Terminal height
    pub height: i32,
    /// Frame buffer
    pub buffer: Vec<Vec<Option<RenderCommand>>>,
}

impl TerminalRenderer {
    /// Create a new terminal renderer
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            buffer: vec![vec![None; width as usize]; height as usize],
        }
    }
}

impl PresentationRenderer for TerminalRenderer {
    fn queue_draw(&mut self, cmd: RenderCommand) {
        let (x, y) = cmd.pos;
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.buffer[y as usize][x as usize] = Some(cmd);
        }
    }

    fn queue_draw_cell(&mut self, _pos: (i32, i32), _cell: &CellKey) {
        // Not needed for terminal output
    }

    fn present(&mut self) {
        for row in &self.buffer {
            for cell in row {
                if let Some(cmd) = cell {
                    print!("{}", cmd.glyph);
                } else {
                    print!(" ");
                }
            }
            println!();
        }
        // Clear buffer for next frame
        for row in &mut self.buffer {
            for cell in row.iter_mut() {
                *cell = None;
            }
        }
    }
}
