use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use engine_core::presentation::ui::layout::grid::GridLayout;
use engine_core::presentation::ui::widget::{EventLogWidget, Panel};
use engine_core::presentation::ui::{
    Alignment, Button, Checkbox, Dropdown, Label, Layout, LayoutDirection, Padding, TextInput,
    UiEvent, UiRoot, UiWidget,
};
use engine_core::systems::job::system::events::{init_job_event_logger, job_event_logger};
use serde_json::json;
use std::sync::{Arc, Mutex};

// ─── Core widget tests ───

#[test]
fn test_label_rendering() {
    let mut renderer = TestRenderer::new();
    let mut label = Label::new("Hello, World!", (2, 3), RenderColor(255, 255, 0));
    label.render(&mut renderer);

    assert_eq!(renderer.draws.len(), 13);
    assert_eq!(renderer.draws[0].glyph, 'H');
    assert_eq!(renderer.draws[0].pos, (2, 3));
    assert_eq!(renderer.draws[12].glyph, '!');
    assert_eq!(renderer.draws[12].pos, (14, 3));
}

#[test]
fn test_button_press_event() {
    let pressed = Arc::new(Mutex::new(false));
    let pressed_clone = pressed.clone();

    let mut button = Button::new(
        "Click Me",
        (5, 5),
        RenderColor(0, 255, 0),
        Some(Box::new(move || {
            *pressed_clone.lock().unwrap() = true;
        })),
        None,
    );

    let mut renderer = TestRenderer::new();
    button.render(&mut renderer);

    let event = UiEvent::Click { x: 5, y: 5 };
    button.handle_event(&event);

    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_panel_composition_and_rendering() {
    let mut renderer = TestRenderer::new();

    let label = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let button = Button::new(
        "B",
        (2, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );

    let mut panel = Panel::new((1, 1));
    panel.add_child(Box::new(label));
    panel.add_child(Box::new(button));

    panel.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (1, 1))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (3, 1))
    );
}

#[test]
fn test_ui_root_event_dispatch() {
    let pressed = Arc::new(Mutex::new(false));
    let pressed_clone = pressed.clone();

    let button = Button::new(
        "OK",
        (0, 0),
        RenderColor(0, 255, 255),
        Some(Box::new(move || {
            *pressed_clone.lock().unwrap() = true;
        })),
        None,
    );

    let mut root = UiRoot::new();
    root.add_child(Box::new(button));

    let event = UiEvent::Click { x: 0, y: 0 };
    root.handle_event(&event);

    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_row_layout_renders_children_in_row() {
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
fn test_column_layout_renders_children_in_column() {
    let mut renderer = TestRenderer::new();

    let label1 = Label::new("X", (0, 0), RenderColor(255, 255, 0));
    let label2 = Label::new("Y", (0, 0), RenderColor(255, 0, 255));

    let mut col = Layout::new(LayoutDirection::Column, (0, 1), 2);
    col.add_child(Box::new(label1));
    col.add_child(Box::new(label2));

    col.render(&mut renderer);

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
fn test_row_layout_event_dispatch() {
    let pressed = Arc::new(Mutex::new(false));
    let pressed_clone = pressed.clone();

    let button1 = Button::new(
        "Btn1",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button2 = Button::new(
        "Btn2",
        (0, 0),
        RenderColor(0, 0, 255),
        Some(Box::new(move || {
            *pressed_clone.lock().unwrap() = true;
        })),
        None,
    );

    let mut row = Layout::new(LayoutDirection::Row, (0, 0), 1);
    row.add_child(Box::new(button1));
    row.add_child(Box::new(button2));

    let event = UiEvent::Click { x: 6, y: 0 };
    row.handle_event(&event);

    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_grid_layout_renders_children_in_grid() {
    let mut renderer = TestRenderer::new();

    let label_a = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label_b = Label::new("B", (0, 0), RenderColor(0, 255, 0));
    let label_c = Label::new("C", (0, 0), RenderColor(0, 0, 255));
    let label_d = Label::new("D", (0, 0), RenderColor(255, 255, 0));

    let mut grid = GridLayout::new((1, 2), (2, 1));
    grid.set_columns(2);
    grid.add_child(Box::new(label_a));
    grid.add_child(Box::new(label_b));
    grid.add_child(Box::new(label_c));
    grid.add_child(Box::new(label_d));

    grid.set_origin((5, 5));
    grid.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (5, 5))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (8, 5))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'C' && cmd.pos == (5, 8))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'D' && cmd.pos == (8, 8))
    );
}

#[test]
fn test_grid_layout_event_dispatch() {
    let pressed = Arc::new(Mutex::new(false));
    let pressed_clone = pressed.clone();

    let button1 = Button::new(
        "X",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button2 = Button::new(
        "Y",
        (0, 0),
        RenderColor(0, 0, 255),
        Some(Box::new(move || {
            *pressed_clone.lock().unwrap() = true;
        })),
        None,
    );

    let mut grid = GridLayout::new((1, 1), (2, 1));
    grid.set_columns(2);
    grid.add_child(Box::new(button1));
    grid.add_child(Box::new(button2));
    grid.set_origin((0, 0));

    let event = UiEvent::Click { x: 3, y: 0 };
    grid.handle_event(&event);

    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_layout_alignment_and_padding() {
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
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (6, 7)),
        "Expected label at (6,7) due to padding and centering"
    );
}

#[test]
fn test_grid_layout_alignment_and_padding() {
    let mut renderer = TestRenderer::new();

    let label_a = Label::new("A", (0, 0), RenderColor(255, 0, 0));
    let label_b = Label::new("B", (0, 0), RenderColor(0, 255, 0));

    let mut grid = GridLayout::new((4, 2), (1, 1));
    grid.set_columns(2);
    grid.set_origin((0, 0));
    grid.set_alignment(Alignment::End);
    grid.set_padding(Padding {
        left: 1,
        right: 2,
        top: 1,
        bottom: 0,
    });
    grid.add_child(Box::new(label_a));
    grid.add_child(Box::new(label_b));

    grid.render(&mut renderer);

    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (1, 1)),
        "Expected label A at (1,1) due to alignment and padding"
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'B' && cmd.pos == (6, 1)),
        "Expected label B at (6,1) due to alignment and padding"
    );
}

#[test]
fn test_text_input_render_and_event() {
    let mut renderer = TestRenderer::new();
    let mut input = TextInput::new((10, 2), 8, RenderColor(0, 255, 255), None);
    input.set_text("hi");

    input.render(&mut renderer);
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
        !renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '|' && cmd.pos == (12, 2))
    );

    input.handle_event(&UiEvent::Click { x: 12, y: 2 });
    input.render(&mut renderer);
    assert!(input.is_focused(), "Input should be focused after click");
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '|' && cmd.pos == (12, 2)),
        "Cursor should be visible after focus"
    );
}

#[test]
fn test_checkbox_render_and_toggle() {
    let mut renderer = TestRenderer::new();
    let toggled = Arc::new(Mutex::new(false));
    let toggled_cb = toggled.clone();

    let mut cb = Checkbox::new("Accept", (5, 1), RenderColor(0, 255, 0), None);
    cb.set_on_toggle(Box::new(move |state| *toggled_cb.lock().unwrap() = state));
    cb.render(&mut renderer);
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '☐'));
    assert!(!*toggled.lock().unwrap());

    cb.handle_event(&UiEvent::Click { x: 5, y: 1 });
    assert!(cb.checked);
    assert!(*toggled.lock().unwrap());

    cb.handle_event(&UiEvent::Click { x: 5, y: 1 });
    assert!(!cb.checked);
    assert!(!*toggled.lock().unwrap());
}

#[test]
fn test_dropdown_render_and_select() {
    let mut renderer = TestRenderer::new();
    let options = vec!["One".to_string(), "Two".to_string(), "Three".to_string()];
    let selected_value = Arc::new(Mutex::new(None));
    let selected_cb = selected_value.clone();

    let mut dropdown = Dropdown::new((10, 5), 10, options.clone(), RenderColor(255, 255, 255));
    dropdown.set_on_select(Box::new(move |val| {
        *selected_cb.lock().unwrap() = Some(val.clone())
    }));

    dropdown.render(&mut renderer);
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '<'));

    dropdown.handle_event(&UiEvent::Click { x: 10, y: 5 });
    assert!(dropdown.expanded);

    dropdown.handle_event(&UiEvent::Click { x: 10, y: 7 });
    assert_eq!(dropdown.selected, Some("Two".to_string()));
    assert_eq!(*selected_value.lock().unwrap(), Some("Two".to_string()));
    assert!(!dropdown.expanded);
}

// ─── Event log widget tests ───

#[test]
fn test_event_log_widget_renders_events() {
    init_job_event_logger();
    let logger = job_event_logger();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let mut widget = EventLogWidget::new(0);
    widget.update();

    let mut renderer = TestRenderer::new();
    widget.render(&mut renderer);

    assert!(
        !renderer.draws.is_empty(),
        "Event log widget should render something"
    );
}

#[test]
fn test_event_log_widget_filters_events() {
    init_job_event_logger();
    let logger = job_event_logger();
    logger.log("job_completed", json!({"id": 1}));
    logger.log("job_failed", json!({"id": 2}));
    logger.log("job_completed", json!({"id": 3}));

    let mut widget = EventLogWidget::new(0);
    widget.set_filter("completed".to_string());
    widget.update();

    let mut renderer = TestRenderer::new();
    widget.render(&mut renderer);

    assert!(
        !renderer.draws.is_empty(),
        "Filtered event log should render something"
    );
}
