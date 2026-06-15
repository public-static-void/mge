use engine_core::presentation::ui::widget::focus_grid::FocusGrid;
use engine_core::presentation::ui::widget::button::Button;
use engine_core::presentation::ui::widget::widget_trait::UiWidget;
use engine_core::presentation::renderer::RenderColor;

fn make_widget(label: &str) -> Box<dyn UiWidget + Send> {
    Box::new(Button::new(
        label,
        (0, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    ))
}

#[test]
fn test_move_focus_right() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 0);
    grid.add_child(make_widget("B"), 1, 0);
    grid.add_child(make_widget("C"), 2, 0);
    // Focus first child
    grid.set_focused_debug(Some(0));
    grid.move_focus_public(1, 0);
    assert_eq!(grid.focused_index(), Some(1));
}

#[test]
fn test_move_focus_down() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 0);
    grid.add_child(make_widget("B"), 1, 0);
    grid.add_child(make_widget("C"), 0, 1);
    grid.set_focused_debug(Some(0));
    grid.move_focus_public(0, 1);
    assert_eq!(grid.focused_index(), Some(2));
}

#[test]
fn test_move_focus_left() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 1, 0);
    grid.add_child(make_widget("B"), 0, 0);
    grid.set_focused_debug(Some(0));
    grid.move_focus_public(-1, 0);
    assert_eq!(grid.focused_index(), Some(1));
}

#[test]
fn test_move_focus_up() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 1);
    grid.add_child(make_widget("B"), 0, 0);
    grid.set_focused_debug(Some(0));
    grid.move_focus_public(0, -1);
    assert_eq!(grid.focused_index(), Some(1));
}

#[test]
fn test_move_focus_edge_no_wrap() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 0);
    grid.add_child(make_widget("B"), 1, 0);
    grid.set_focused_debug(Some(0));
    // Try to move left from column 0 — no candidate in that direction
    grid.move_focus_public(-1, 0);
    assert_eq!(grid.focused_index(), Some(0)); // stays at current
}

#[test]
fn test_move_focus_no_children() {
    let mut grid = FocusGrid::new(3, 3);
    grid.move_focus_public(1, 0);
    assert_eq!(grid.focused_index(), None);
}

#[test]
fn test_move_focus_first_child_when_none_focused() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 0);
    grid.add_child(make_widget("B"), 1, 0);
    grid.move_focus_public(1, 0);
    // When nothing focused, focuses first child
    assert_eq!(grid.focused_index(), Some(0));
}

#[test]
fn test_move_focus_selects_nearest() {
    let mut grid = FocusGrid::new(3, 3);
    grid.add_child(make_widget("A"), 0, 0);
    grid.add_child(make_widget("B"), 2, 0);
    grid.add_child(make_widget("C"), 1, 0);
    grid.set_focused_debug(Some(0));
    // Moving right: both B (col 2) and C (col 1) are to the right.
    // C is closest, so should be selected.
    grid.move_focus_public(1, 0);
    assert_eq!(grid.focused_index(), Some(2)); // C at index 2
}
