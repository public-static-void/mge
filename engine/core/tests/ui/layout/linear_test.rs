use engine_core::presentation::ui::widget::Label;
use engine_core::presentation::ui::layout::linear::Layout;
use engine_core::presentation::ui::layout::direction::{Alignment, LayoutDirection, Padding};
use engine_core::presentation::renderer::{RenderColor, TestRenderer};

#[test]
fn test_row_positions_children_horizontally() {
    let mut renderer = TestRenderer::new();
    let label1 = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label2 = Label::new("B", (0, 0), RenderColor(0, 255, 0));
    let label3 = Label::new("C", (0, 0), RenderColor(0, 0, 255));

    let mut row = Layout::new(LayoutDirection::Row, (2, 5), 1);
    row.add_child(Box::new(label1));
    row.add_child(Box::new(label2));
    row.add_child(Box::new(label3));

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (2, 5))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (4, 5))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'C' && cmd.pos == (6, 5))
    );
}

#[test]
fn test_column_positions_children_vertically() {
    let mut renderer = TestRenderer::new();
    let label1 = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let label2 = Label::new("Y", (0, 0), RenderColor(0, 255, 0));

    let mut col = Layout::new(LayoutDirection::Column, (0, 1), 2);
    col.add_child(Box::new(label1));
    col.add_child(Box::new(label2));

    col.render(&mut renderer);

    // Column: X at (0,1), Y at (0,4) — spacing 2 between rows
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (0, 1))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'Y' && cmd.pos == (0, 4))
    );
}

#[test]
fn test_alignment_center_offsets_children() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let mut row = Layout::new(LayoutDirection::Row, (2, 5), 1);
    row.add_child(Box::new(label));
    row.set_alignment(Alignment::Center);
    row.set_padding(Padding::uniform(2));

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (6, 7))
    );
}

#[test]
fn test_alignment_end_offsets_children() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 0);
    row.add_child(Box::new(label));
    row.set_alignment(Alignment::End);
    row.set_padding(Padding::uniform(2));

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (6, 2))
    );
}

#[test]
fn test_alignment_start_no_offset() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 0);
    row.add_child(Box::new(label));
    row.set_alignment(Alignment::Start);

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (0, 0))
    );
}

#[test]
fn test_spacing_between_children() {
    let mut renderer = TestRenderer::new();
    let label1 = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label2 = Label::new("B", (0, 0), RenderColor(0, 255, 0));

    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 5);
    row.add_child(Box::new(label1));
    row.add_child(Box::new(label2));

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (0, 0))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (6, 0))
    );
}

#[test]
fn test_zero_spacing_no_gap() {
    let mut renderer = TestRenderer::new();
    let label1 = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label2 = Label::new("B", (0, 0), RenderColor(0, 255, 0));

    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 0);
    row.add_child(Box::new(label1));
    row.add_child(Box::new(label2));

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (0, 0))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (1, 0))
    );
}

#[test]
fn test_padding_applied_correctly() {
    let mut renderer = TestRenderer::new();
    let label = Label::new("X", (0, 0), RenderColor(255, 0, 0));
    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 1);
    row.add_child(Box::new(label));
    row.set_padding(Padding {
        left: 3,
        right: 0,
        top: 2,
        bottom: 0,
    });

    row.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (3, 2))
    );
}
