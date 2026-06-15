use engine_core::presentation::ui::layout::grid::GridLayout;
use engine_core::presentation::ui::layout::direction::{Alignment, Padding};
use engine_core::presentation::renderer::RenderColor;
use engine_core::presentation::ui::widget::Label;

#[test]
fn test_grid_size_with_children() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "C",
        (0, 0),
        RenderColor(0, 0, 255),
    )));
    assert_eq!(grid.grid_size(), (2, 2)); // 2 cols, 2 rows (3 children in 2 cols = 2 rows)
}

#[test]
fn test_grid_size_single_child() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(3);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    assert_eq!(grid.grid_size(), (3, 1)); // 3 cols, 1 row
}

#[test]
fn test_grid_size_no_children() {
    let grid = GridLayout::new((4, 2), (1, 1));
    assert_eq!(grid.grid_size(), (1, 0)); // 1 col (default), 0 rows
}

#[test]
fn test_child_position_basic() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((5, 5));
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "C",
        (0, 0),
        RenderColor(0, 0, 255),
    )));
    // 2 columns => A(0) at col0/row0, B(1) at col1/row0, C(2) at col0/row1
    // cell_size=(4,2), spacing=(1,1), origin=(5,5)
    // A: (5 + 0, 5 + 0) = (5, 5)
    // B: (5 + 1*(4+1), 5 + 0) = (10, 5)
    // C: (5 + 0, 5 + 1*(2+1)) = (5, 8)
    assert_eq!(grid.child_position(0), (5, 5));
    assert_eq!(grid.child_position(1), (10, 5));
    assert_eq!(grid.child_position(2), (5, 8));
}

#[test]
fn test_child_position_with_padding() {
    let mut grid = GridLayout::new((3, 3), (0, 0));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.set_padding(Padding {
        left: 2,
        right: 2,
        top: 1,
        bottom: 1,
    });
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    // A: origin(0) + padding_left(2) = (2, 1)
    assert_eq!(grid.child_position(0), (2, 1));
}

#[test]
fn test_child_position_alignment_center() {
    let mut grid = GridLayout::new((4, 2), (0, 0));
    grid.set_columns(1);
    grid.set_origin((0, 0));
    grid.set_alignment(Alignment::Center);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    // widget_area = padding(0,0,0,0) + grid_area(4,2) = (4,2)
    // alignment_offset: Center => ((4-4)/2, (2-2)/2) = (0, 0)
    // A: (0 + 0 + 0, 0 + 0 + 0) = (0, 0)
    assert_eq!(grid.child_position(0), (0, 0));
}

#[test]
fn test_child_at_hit() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    // A at (0,0), B at (5,0) — cell (0,0) -> A, cell (5,0) -> B
    assert_eq!(grid.child_at(0, 0), Some(0));
    assert_eq!(grid.child_at(5, 0), Some(1));
}

#[test]
fn test_child_at_miss_out_of_bounds() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    // Click at negative position or far outside bounds
    assert_eq!(grid.child_at(-1, 0), None);
    assert_eq!(grid.child_at(100, 100), None);
    // Click outside cell area (between cells)
    assert_eq!(grid.child_at(4, 0), None);
    // Click below valid row
    assert_eq!(grid.child_at(0, 3), None);
}

#[test]
fn test_child_at_between_cells() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    // Click in spacing area (x=4, y=0) is between cell(0,0) and cell(1,0)
    assert_eq!(grid.child_at(4, 0), None);
}

#[test]
fn test_alignment_offset_start() {
    let mut grid = GridLayout::new((4, 2), (0, 0));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.set_alignment(Alignment::Start);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    // Alignment::Start => offset (0, 0)
    assert_eq!(grid.alignment_offset(), (0, 0));
}

#[test]
fn test_alignment_offset_center() {
    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.set_padding(Padding {
        left: 0,
        right: 2,
        top: 0,
        bottom: 2,
    });
    grid.set_alignment(Alignment::Center);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    // grid_area with 2 cols (2 children), cell=(4,2), spacing=(1,1)
    // grid_w = 2*4 + 1*1 = 9, grid_h = 1*2 + 0*1 = 2
    // widget_area = padding(0+2, 0+2) => w=11, h=4
    // alignment_offset: Center => ((11-9)/2, (4-2)/2) = (1, 1)
    assert_eq!(grid.alignment_offset(), (1, 1));
}

#[test]
fn test_alignment_offset_end() {
    let mut grid = GridLayout::new((4, 2), (0, 0));
    grid.set_columns(2);
    grid.set_alignment(Alignment::End);
    grid.add_child(Box::new(Label::new(
        "A",
        (0, 0),
        RenderColor(255, 0, 0),
    )));
    grid.add_child(Box::new(Label::new(
        "B",
        (0, 0),
        RenderColor(0, 255, 0),
    )));
    // Alignment::End => offset (0, 0)
    assert_eq!(grid.alignment_offset(), (0, 0));
}
