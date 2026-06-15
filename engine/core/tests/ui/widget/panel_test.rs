use engine_core::presentation::ui::widget::panel::Panel;
use engine_core::presentation::ui::widget::Label;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};

#[test]
fn test_panel_offsets_child_by_position() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let mut panel = Panel::new((1, 1));
    panel.add_child(Box::new(label));
    panel.render(&mut renderer);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (1, 1))
    );
}

#[test]
fn test_panel_empty_renders_nothing() {
    let mut renderer = TestRenderer::new();
    let mut panel = Panel::new((0, 0));
    panel.render(&mut renderer);
    assert_eq!(renderer.draws.len(), 0);
}

#[test]
fn test_nested_panels_compound_offset() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let mut inner = Panel::new((3, 3));
    inner.add_child(Box::new(label));
    let mut outer = Panel::new((5, 5));
    outer.add_child(Box::new(inner));
    outer.render(&mut renderer);
    // Inner panel at (3,3) + outer panel at (5,5) = label at (8, 8)
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (8, 8))
    );
}

#[test]
fn test_panel_multiple_children_all_offset() {
    let mut renderer = TestRenderer::new();
    let label_a = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label_b = Label::new("B", (2, 0), RenderColor(0, 255, 0));
    let mut panel = Panel::new((10, 10));
    panel.add_child(Box::new(label_a));
    panel.add_child(Box::new(label_b));
    panel.render(&mut renderer);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (10, 10))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (12, 10))
    );
}

#[test]
fn test_panel_triple_nested() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("Z", (0, 0), RenderColor(255, 255, 255));
    let mut inner1 = Panel::new((2, 2));
    inner1.add_child(Box::new(label));
    let mut inner2 = Panel::new((3, 3));
    inner2.add_child(Box::new(inner1));
    let mut outer = Panel::new((5, 5));
    outer.add_child(Box::new(inner2));
    outer.render(&mut renderer);
    // 5+3+2 = 10
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'Z' && cmd.pos == (10, 10))
    );
}
