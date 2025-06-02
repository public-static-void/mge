use engine_core::presentation::renderer::RenderColor;
use engine_core::presentation::ui::root::{Direction, UiRoot};
use engine_core::presentation::ui::widget::button::Button;
use engine_core::presentation::ui::widget::checkbox::Checkbox;
use engine_core::presentation::ui::widget::focus_grid::FocusGrid;
use engine_core::presentation::ui::widget::text_input::TextInput;

#[test]
fn test_physical_navigation_right_and_down() {
    let mut root = UiRoot::new();

    // Place three buttons in a triangle
    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button3 = Button::new(
        "C",
        (0, 10),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));
    root.add_child(Box::new(button3));

    // Focus first button (A)
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));

    // Move right: should go to B (rightmost)
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1));

    // Move down: from B, should go to C (closest below, even if not aligned)
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(2));

    // Move left: from C, should stay at C (no widget to the left)
    root.focus_physical(Direction::Left);
    assert_eq!(root.focused_index(), Some(2));
}

#[test]
fn test_focus_groups_restrict_navigation() {
    let mut root = UiRoot::new();

    // Two groups: group 1 and group 2
    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(1),
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(2),
    );
    let input1 = TextInput::new((0, 10), 8, RenderColor(0, 255, 255), Some(1));
    let cb2 = Checkbox::new("C", (10, 10), RenderColor(0, 255, 0), Some(2));

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));
    root.add_child(Box::new(input1));
    root.add_child(Box::new(cb2));

    // Set focus group to 1
    root.set_focus_group(Some(1));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0)); // Button A
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(2)); // TextInput (only other in group 1)

    // Set focus group to 2
    root.set_focus_group(Some(2));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1)); // Button B
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(3)); // Checkbox C

    // Set focus group to None (all widgets focusable)
    root.set_focus_group(None);
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0)); // Button A
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1)); // Button B
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(3)); // Checkbox C
}

#[test]
fn test_focus_group_exclusion() {
    let mut root = UiRoot::new();

    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(1),
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(2),
    );

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));

    root.set_focus_group(Some(1));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));
    root.focus_physical(Direction::Right);
    // Should stay on the same button, as no other in group
    assert_eq!(root.focused_index(), Some(0));
}

#[test]
fn test_grid_layout_navigation() {
    // The FocusGrid::new method takes two i32 arguments: columns, rows
    let mut grid = FocusGrid::new(2, 2);

    // Add children at specific grid positions (col, row)
    grid.add_child(
        Box::new(Button::new(
            "A",
            (0, 0),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        0,
        0,
    );
    grid.add_child(
        Box::new(Button::new(
            "B",
            (1, 0),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        1,
        0,
    );
    grid.add_child(
        Box::new(Button::new(
            "C",
            (0, 1),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        0,
        1,
    );
    grid.add_child(
        Box::new(Button::new(
            "D",
            (1, 1),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        1,
        1,
    );

    // Start focus at A (top-left)
    grid.move_focus_public(0, 0);
    assert_eq!(grid.focused_index(), Some(0));

    // Move right: A -> B
    grid.move_focus_public(1, 0);
    assert_eq!(grid.focused_index(), Some(1));

    // Move down: B -> D
    grid.move_focus_public(0, 1);
    assert_eq!(grid.focused_index(), Some(3));

    // Move left: D -> C
    grid.move_focus_public(-1, 0);
    assert_eq!(grid.focused_index(), Some(2));

    // Move up: C -> A
    grid.move_focus_public(0, -1);
    assert_eq!(grid.focused_index(), Some(0));
}
