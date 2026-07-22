use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::presentation::PresentationSystem;
use engine_core::presentation::renderer::{
    PresentationRenderer, RenderColor, RenderCommand, TerminalRenderer, TestRenderer,
};
use serde_json::json;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ─── Presentation system test ───

#[test]
fn test_presentation_system_renders_entities() {
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

    let entity = world.spawn_entity();
    world
        .set_component(
            entity,
            "Position",
            json!({
                "pos": { "Square": { "x": 1, "y": 2, "z": 0 } }
            }),
        )
        .unwrap();
    world
        .set_component(
            entity,
            "Renderable",
            json!({
                "glyph": "@",
                "color": [255, 255, 255]
            }),
        )
        .unwrap();

    let renderer = TestRenderer::new();
    let mut system = PresentationSystem::new(renderer);
    system.render_world(&world);

    assert_eq!(system.renderer.draws.len(), 1);
    assert_eq!(system.renderer.draws[0].glyph, '@');
    assert_eq!(system.renderer.draws[0].pos, (1, 2));
    assert_eq!(system.renderer.draws[0].color, RenderColor(255, 255, 255));
}

// ─── Terminal renderer ANSI tests ───

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
    let saved_fd = unsafe { dup(stdout_fd) };
    assert!(saved_fd >= 0, "dup failed");

    let ret = unsafe { dup2(write_fd, stdout_fd) };
    assert!(ret >= 0, "dup2 failed");

    f();

    let ret = unsafe { dup2(saved_fd, stdout_fd) };
    assert!(ret >= 0, "dup2 restore failed");
    unsafe { close(saved_fd) };

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

    assert!(
        output.contains("\x1b[38;2;255;0;0m@\x1b[0m"),
        "expected red ANSI escape for glyph @, got: {output:?}"
    );
    assert!(
        output.contains("\x1b[38;2;0;255;0mB\x1b[0m"),
        "expected green ANSI escape for glyph B, got: {output:?}"
    );
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
