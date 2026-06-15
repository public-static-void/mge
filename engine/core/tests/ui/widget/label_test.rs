use engine_core::presentation::ui::widget::Label;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};

#[test]
fn test_label_creation_with_text_position_color() {
    let label = Label::new("Hello", (5, 10), RenderColor(255, 0, 0));
    assert_eq!(label.text, "Hello");
    assert_eq!(label.pos, (5, 10));
    assert_eq!(label.color, RenderColor(255, 0, 0));
}

#[test]
fn test_label_render_produces_correct_commands() {
    let mut renderer = TestRenderer::new();
    let mut label = Label::new("Hi", (2, 3), RenderColor(0, 255, 0));
    label.render(&mut renderer);

    assert_eq!(renderer.draws.len(), 2);
    assert_eq!(renderer.draws[0].glyph, 'H');
    assert_eq!(renderer.draws[0].pos, (2, 3));
    assert_eq!(renderer.draws[0].color, RenderColor(0, 255, 0));
    assert_eq!(renderer.draws[1].glyph, 'i');
    assert_eq!(renderer.draws[1].pos, (3, 3));
}

#[test]
fn test_label_empty_string_renders_nothing() {
    let mut renderer = TestRenderer::new();
    let mut label = Label::new("", (0, 0), RenderColor(255, 255, 255));
    label.render(&mut renderer);
    assert_eq!(renderer.draws.len(), 0);
}

#[test]
fn test_label_color_application() {
    let mut renderer = TestRenderer::new();
    let mut label = Label::new("A", (0, 0), RenderColor(100, 150, 200));
    label.render(&mut renderer);
    assert_eq!(renderer.draws[0].color, RenderColor(100, 150, 200));
}

#[test]
fn test_label_long_text_renders_all_chars() {
    let mut renderer = TestRenderer::new();
    let text = "ABCDE";
    let mut label = Label::new(text, (1, 1), RenderColor(255, 255, 255));
    label.render(&mut renderer);
    assert_eq!(renderer.draws.len(), 5);
    for (i, ch) in text.chars().enumerate() {
        assert_eq!(renderer.draws[i].glyph, ch);
        assert_eq!(renderer.draws[i].pos, (1 + i as i32, 1));
    }
}
