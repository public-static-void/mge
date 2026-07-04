use engine_core::presentation::renderer::{
    PresentationRenderer, RenderColor, RenderCommand, TerminalRenderer,
};
use std::fs::File;
use std::io::{Read, Write};
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

    // Flush stdout before redirecting to prevent stale buffered data from
    // earlier test output from leaking into the capture pipe.
    std::io::stdout().flush().ok();

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
    assert!(
        output.is_empty(),
        "zero-size buffer should produce no output, got: {output:?}"
    );
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
