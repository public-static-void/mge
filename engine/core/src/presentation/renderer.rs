//! Renderer abstraction for the presentation layer.

use crate::map::cell_key::CellKey;
use serde::{Deserialize, Serialize};

/// A color in RGB space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderColor(pub u8, pub u8, pub u8);

/// Named color constants matching common terminal colors.
/// These are emitted as ANSI 24-bit foreground escapes;
/// no fallback for legacy terminals is provided (NFR001).
pub const COLOR_WHITE: RenderColor = RenderColor(255, 255, 255);
pub const COLOR_BLACK: RenderColor = RenderColor(0, 0, 0);
pub const COLOR_RED: RenderColor = RenderColor(255, 0, 0);
pub const COLOR_GREEN: RenderColor = RenderColor(0, 255, 0);
pub const COLOR_YELLOW: RenderColor = RenderColor(255, 255, 0);
pub const COLOR_BLUE: RenderColor = RenderColor(0, 0, 255);
pub const COLOR_GRAY: RenderColor = RenderColor(128, 128, 128);
pub const COLOR_DIM_GRAY: RenderColor = RenderColor(60, 60, 60);

/// Very dim color for cells outside the visible set.
pub const COLOR_VERY_DIM: RenderColor = RenderColor(25, 25, 25);

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
        // Uses BufWriter + stdout().lock() to avoid per-cell heap allocation (R003/AC007).
        // ANSI 24-bit escapes are always emitted; no fallback for legacy terminals (NFR001).
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        for row in &self.buffer {
            for cell in row {
                match cell {
                    Some(cmd) => {
                        let r = cmd.color.0;
                        let g = cmd.color.1;
                        let b = cmd.color.2;
                        write!(out, "\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, cmd.glyph)
                            .expect("write to stdout");
                    }
                    None => {
                        write!(out, " ").expect("write to stdout");
                    }
                }
            }
            writeln!(out).expect("write newline to stdout");
        }
        out.flush().expect("flush stdout");
        // Clear buffer for next frame
        for row in &mut self.buffer {
            for cell in row.iter_mut() {
                *cell = None;
            }
        }
    }
}
