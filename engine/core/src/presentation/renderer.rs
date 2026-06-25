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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
    use std::sync::Mutex;

    /// Serializes stdout-capturing tests to prevent interference from parallel execution.
    static STDOUT_CAPTURE_LOCK: Mutex<()> = Mutex::new(());

    unsafe extern "C" {
        fn dup(fd: std::os::raw::c_int) -> std::os::raw::c_int;
        fn dup2(oldfd: std::os::raw::c_int, newfd: std::os::raw::c_int) -> std::os::raw::c_int;
        fn close(fd: std::os::raw::c_int) -> std::os::raw::c_int;
        fn pipe(pipefd: *mut std::os::raw::c_int) -> std::os::raw::c_int;
    }

    /// Redirect stdout to a pipe, invoke `f`, restore stdout, return captured output.
    /// Serialized via STDOUT_CAPTURE_LOCK to prevent parallel test interference.
    fn capture_stdout<F: FnOnce()>(f: F) -> String {
        let _guard = STDOUT_CAPTURE_LOCK.lock().expect("stdout capture lock");

        let mut fds = [0i32; 2];
        let ret = unsafe { pipe(fds.as_mut_ptr()) };
        assert_eq!(ret, 0, "pipe creation failed");
        let [read_fd, write_fd] = fds;
        let mut rx = unsafe { File::from_raw_fd(read_fd) };
        let tx = unsafe { OwnedFd::from_raw_fd(write_fd) };

        let stdout_fd = std::io::stdout().as_raw_fd();

        // Duplicate original stdout fd for restoration
        let saved_fd = unsafe { dup(stdout_fd) };
        assert!(saved_fd >= 0, "dup failed");

        // Replace stdout with write end of our pipe
        let ret = unsafe { dup2(write_fd, stdout_fd) };
        assert!(ret >= 0, "dup2 failed");

        // Run the function (writes to our pipe instead of terminal)
        f();

        // Restore original stdout
        let ret = unsafe { dup2(saved_fd, stdout_fd) };
        assert!(ret >= 0, "dup2 restore failed");
        unsafe { close(saved_fd) };

        // Drop tx (close write end) so read_to_string won't block
        drop(tx);

        let mut output = String::new();
        rx.read_to_string(&mut output).expect("read from pipe");
        output
    }

    #[test]
    fn test_terminal_renderer_ansi_output() {
        let mut renderer = TerminalRenderer::new(3, 1);
        renderer.queue_draw(RenderCommand {
            glyph: '@',
            color: RenderColor(255, 0, 0),
            pos: (0, 0),
        });
        renderer.queue_draw(RenderCommand {
            glyph: 'B',
            color: RenderColor(0, 255, 0),
            pos: (1, 0),
        });

        let output = capture_stdout(|| renderer.present());

        // Red glyph wrapped in ANSI 24-bit foreground escape
        assert!(
            output.contains("\x1b[38;2;255;0;0m@\x1b[0m"),
            "expected red ANSI escape for glyph @, got: {output:?}"
        );
        // Green glyph independently wrapped
        assert!(
            output.contains("\x1b[38;2;0;255;0mB\x1b[0m"),
            "expected green ANSI escape for glyph B, got: {output:?}"
        );
        // None cell emits bare space with no escapes
        assert!(
            output.contains(" "),
            "output should contain spaces for None cells, got: {output:?}"
        );
    }

    #[test]
    fn test_terminal_renderer_ansi_black_color() {
        let mut renderer = TerminalRenderer::new(1, 1);
        renderer.queue_draw(RenderCommand {
            glyph: 'X',
            color: RenderColor(0, 0, 0),
            pos: (0, 0),
        });

        let output = capture_stdout(|| renderer.present());

        assert!(
            output.contains("\x1b[38;2;0;0;0mX\x1b[0m"),
            "expected black ANSI escape, got: {output:?}"
        );
    }

    #[test]
    fn test_terminal_renderer_ansi_all_none() {
        let mut renderer = TerminalRenderer::new(2, 2);
        let output = capture_stdout(|| renderer.present());

        assert!(
            !output.contains("\x1b["),
            "no ANSI escapes expected for empty buffer, got: {output:?}"
        );
        assert_eq!(
            output.matches(' ').count(),
            4,
            "four spaces for 2x2 empty buffer"
        );
        assert_eq!(output.matches('\n').count(), 2, "two newlines for 2 rows");
    }

    #[test]
    fn test_terminal_renderer_ansi_zero_size() {
        let mut renderer = TerminalRenderer::new(0, 0);
        let output = capture_stdout(|| renderer.present());
        assert!(output.is_empty(), "zero-size buffer produces no output");
    }

    #[test]
    fn test_terminal_renderer_ansi_reset_per_glyph() {
        let mut renderer = TerminalRenderer::new(2, 1);
        renderer.queue_draw(RenderCommand {
            glyph: 'A',
            color: RenderColor(255, 0, 0),
            pos: (0, 0),
        });
        renderer.queue_draw(RenderCommand {
            glyph: 'B',
            color: RenderColor(0, 255, 0),
            pos: (1, 0),
        });

        let output = capture_stdout(|| renderer.present());

        // Each glyph independently wrapped: reset after A, escape before B
        assert!(
            output.contains("\x1b[0m\x1b[38"),
            "adjacent colored glyphs should each be wrapped: {output:?}"
        );
    }

    #[test]
    fn test_terminal_renderer_buffer_cleared_after_present() {
        let mut renderer = TerminalRenderer::new(1, 1);
        renderer.queue_draw(RenderCommand {
            glyph: '@',
            color: RenderColor(255, 0, 0),
            pos: (0, 0),
        });

        let _first = capture_stdout(|| renderer.present());

        let second = capture_stdout(|| renderer.present());
        assert!(
            !second.contains("\x1b["),
            "buffer should be cleared after present, got: {second:?}"
        );
    }
}
