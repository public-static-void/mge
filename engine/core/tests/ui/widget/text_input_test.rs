use engine_core::presentation::ui::widget::text_input::TextInput;
use engine_core::presentation::ui::UiEvent;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};

#[test]
fn test_text_input_set_text_get_text_roundtrip() {
    let mut input = TextInput::new((0, 0), 10, RenderColor(255, 255, 255), None);
    assert_eq!(input.text, "");
    input.set_text("hello");
    assert_eq!(input.text, "hello");
}

#[test]
fn test_text_input_cursor_updates_on_set_text() {
    let mut input = TextInput::new((0, 0), 10, RenderColor(255, 255, 255), None);
    assert_eq!(input.cursor, 0);
    input.set_text("hello");
    assert_eq!(input.cursor, 5);
}

#[test]
fn test_text_input_render_produces_draw_commands() {
    let mut renderer = TestRenderer::new();
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.set_text("hi");
    input.render(&mut renderer);

    // Width is 8, so 8 glyphs: 'h', 'i', '_' x 6
    assert_eq!(renderer.draws.len(), 8);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'h' && cmd.pos == (10, 2))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'i' && cmd.pos == (11, 2))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '_' && cmd.pos == (12, 2))
    );
}

#[test]
fn test_text_input_click_to_focus() {
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    assert!(!input.focused);
    input.handle_event(&UiEvent::Click { x: 10, y: 2 });
    assert!(input.focused);
}

#[test]
fn test_text_input_click_outside_loses_focus() {
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.focused = true;
    input.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!input.focused);
}

#[test]
fn test_text_input_cursor_position_after_click() {
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.set_text("hello");
    // Click at x=12 (position 2 in text)
    input.handle_event(&UiEvent::Click { x: 12, y: 2 });
    assert_eq!(input.cursor, 2);
}

#[test]
fn test_text_input_cursor_clamped_to_text_length() {
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.set_text("hi");
    // Click beyond text length
    input.handle_event(&UiEvent::Click { x: 20, y: 2 });
    assert_eq!(input.cursor, 2); // clamped to text.len()
}

#[test]
fn test_text_input_focused_shows_cursor() {
    let mut renderer = TestRenderer::new();
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.set_text("hi");
    input.focused = true;
    input.cursor = 2;
    input.render(&mut renderer);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '|' && cmd.pos == (12, 2))
    );
}
