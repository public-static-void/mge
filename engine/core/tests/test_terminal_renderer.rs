use engine_core::presentation::renderer::{
    PresentationRenderer, RenderColor, RenderCommand, TerminalRenderer,
};

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

    let mut buf = Vec::new();
    renderer.present_to(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

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

    let mut buf = Vec::new();
    renderer.present_to(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert!(
        output.contains("\x1b[38;2;0;0;0mX\x1b[0m"),
        "expected black ANSI escape, got: {output:?}"
    );
}

#[test]
fn test_terminal_renderer_ansi_all_none() {
    let mut renderer = TerminalRenderer::new(2, 2);
    let mut buf = Vec::new();
    renderer.present_to(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

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
    let mut buf = Vec::new();
    renderer.present_to(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
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

    let mut buf = Vec::new();
    renderer.present_to(&mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

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

    let mut first = Vec::new();
    renderer.present_to(&mut first).unwrap();
    let mut second = Vec::new();
    renderer.present_to(&mut second).unwrap();
    let second = String::from_utf8(second).unwrap();
    assert!(
        !second.contains("\x1b["),
        "buffer should be cleared after present, got: {second:?}"
    );
}
