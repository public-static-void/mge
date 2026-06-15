use engine_core::presentation::ui::root::{UiRoot, Direction, is_in_direction, navigation_score};
use engine_core::presentation::renderer::RenderColor;
use engine_core::presentation::ui::widget::Button;

#[test]
fn test_is_in_direction_right() {
    assert!(is_in_direction((0, 0), (1, 0), Direction::Right));
    assert!(!is_in_direction((1, 0), (0, 0), Direction::Right));
    assert!(!is_in_direction((0, 0), (0, 1), Direction::Right)); // same x
}

#[test]
fn test_is_in_direction_left() {
    assert!(is_in_direction((1, 0), (0, 0), Direction::Left));
    assert!(!is_in_direction((0, 0), (1, 0), Direction::Left));
}

#[test]
fn test_is_in_direction_down() {
    assert!(is_in_direction((0, 0), (0, 1), Direction::Down));
    assert!(!is_in_direction((0, 1), (0, 0), Direction::Down));
}

#[test]
fn test_is_in_direction_up() {
    assert!(is_in_direction((0, 1), (0, 0), Direction::Up));
    assert!(!is_in_direction((0, 0), (0, 1), Direction::Up));
}

#[test]
fn test_navigation_score_prefers_primary_axis() {
    // For Right direction: dx.abs + dy.abs * 2
    let score = navigation_score((0, 0), (3, 1), Direction::Right);
    // dx = 3, dy = 1 => 3 + 1*2 = 5
    assert!((score - 5.0).abs() < f32::EPSILON);
}

#[test]
fn test_navigation_score_penalizes_perpendicular() {
    // For Right direction, vertical distance is penalized (multiplied by 2)
    let aligned = navigation_score((0, 0), (3, 0), Direction::Right);
    let offset = navigation_score((0, 0), (3, 2), Direction::Right);
    // 3 + 0*2 = 3 vs 3 + 2*2 = 7
    assert!(aligned < offset);
}

#[test]
fn test_navigation_score_down() {
    // For Down direction: dy.abs + dx.abs * 2
    let score = navigation_score((1, 0), (1, 4), Direction::Down);
    // dy = 4, dx = 0 => 4 + 0*2 = 4
    assert!((score - 4.0).abs() < f32::EPSILON);
}

#[test]
fn test_root_empty_children_focus_physical_noop() {
    let mut root = UiRoot::new();
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), None);
}

#[test]
fn test_root_focus_physical_selects_first_when_none_focused() {
    let mut root = UiRoot::new();
    root.add_child(Box::new(Button::new(
        "A",
        (0, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.add_child(Box::new(Button::new(
        "B",
        (5, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.focus_physical(Direction::Right);
    // When nothing focused, picks first focusable child
    assert_eq!(root.focused_index(), Some(0));
}

#[test]
fn test_root_focus_physical_moves_right() {
    let mut root = UiRoot::new();
    root.add_child(Box::new(Button::new(
        "A",
        (0, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.add_child(Box::new(Button::new(
        "B",
        (5, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.set_focused_index(Some(0)); // manually set focus to first
    root.focus_physical(Direction::Right);
    // B is to the right of A, so should move to B (index 1)
    assert_eq!(root.focused_index(), Some(1));
}

#[test]
fn test_root_focus_physical_no_widget_in_direction_stays() {
    let mut root = UiRoot::new();
    root.add_child(Box::new(Button::new(
        "A",
        (5, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.add_child(Box::new(Button::new(
        "B",
        (10, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.set_focused_index(Some(0));
    // A is at (5,0), B is at (10,0) — both to the right of origin but B is to the right of A
    root.focus_physical(Direction::Left);
    // No widget to the left of A (B is to the right), so focus stays
    assert_eq!(root.focused_index(), Some(0));
}

#[test]
fn test_root_focus_physical_no_focusable_children() {
    let mut root = UiRoot::new();
    // Panel is not focusable
    root.add_child(Box::new(engine_core::presentation::ui::widget::Panel::new((
        0, 0,
    ))));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), None);
}

#[test]
fn test_root_clear_focus() {
    let mut root = UiRoot::new();
    root.add_child(Box::new(Button::new(
        "A",
        (0, 0),
        RenderColor(255, 255, 255),
        None,
        None,
    )));
    root.set_focused_index(Some(0));
    root.clear_focus();
    assert_eq!(root.focused_index(), None);
}
