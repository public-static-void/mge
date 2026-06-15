use engine_core::presentation::ui::widget::button::Button;
use engine_core::presentation::ui::widget::widget_trait::{UiWidget, WidgetCallback};
use engine_core::presentation::ui::UiEvent;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use std::sync::{Arc, Mutex};

#[test]
fn test_button_click_fires_on_press() {
    let pressed = Arc::new(Mutex::new(false));
    let cb = pressed.clone();
    let mut btn = Button::new(
        "Test",
        (0, 0),
        RenderColor(255, 255, 255),
        Some(Box::new(move || *cb.lock().unwrap() = true)),
        None,
    );
    btn.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_button_click_outside_bounds_no_callback() {
    let pressed = Arc::new(Mutex::new(false));
    let cb = pressed.clone();
    let mut btn = Button::new(
        "Test",
        (5, 5),
        RenderColor(255, 255, 255),
        Some(Box::new(move || *cb.lock().unwrap() = true)),
        None,
    );
    btn.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!*pressed.lock().unwrap());
}

#[test]
fn test_button_keypress_enter_fires_when_focused() {
    let pressed = Arc::new(Mutex::new(false));
    let cb = pressed.clone();
    let mut btn = Button::new(
        "Test",
        (0, 0),
        RenderColor(255, 255, 255),
        Some(Box::new(move || *cb.lock().unwrap() = true)),
        None,
    );
    btn.set_focused(true);
    btn.handle_event(&UiEvent::KeyPress {
        key: "Enter".to_string(),
    });
    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_button_keypress_space_fires_when_focused() {
    let pressed = Arc::new(Mutex::new(false));
    let cb = pressed.clone();
    let mut btn = Button::new(
        "Test",
        (0, 0),
        RenderColor(255, 255, 255),
        Some(Box::new(move || *cb.lock().unwrap() = true)),
        None,
    );
    btn.set_focused(true);
    btn.handle_event(&UiEvent::KeyPress {
        key: "Space".to_string(),
    });
    assert!(*pressed.lock().unwrap());
}

#[test]
fn test_button_keypress_not_focused_does_nothing() {
    let pressed = Arc::new(Mutex::new(false));
    let cb = pressed.clone();
    let mut btn = Button::new(
        "Test",
        (0, 0),
        RenderColor(255, 255, 255),
        Some(Box::new(move || *cb.lock().unwrap() = true)),
        None,
    );
    btn.handle_event(&UiEvent::KeyPress {
        key: "Enter".to_string(),
    });
    assert!(!*pressed.lock().unwrap());
}

#[test]
fn test_button_render_produces_draw_commands() {
    let mut renderer = TestRenderer::new();
    let mut btn = Button::new("AB", (2, 3), RenderColor(255, 0, 0), None, None);
    btn.render(&mut renderer);

    assert_eq!(renderer.draws.len(), 2);
    assert_eq!(renderer.draws[0].glyph, 'A');
    assert_eq!(renderer.draws[0].pos, (2, 3));
    assert_eq!(renderer.draws[0].color, RenderColor(255, 0, 0));
    assert_eq!(renderer.draws[1].glyph, 'B');
    assert_eq!(renderer.draws[1].pos, (3, 3));
}

#[test]
fn test_button_empty_label_renders_nothing() {
    let mut renderer = TestRenderer::new();
    let mut btn = Button::new("", (0, 0), RenderColor(255, 255, 255), None, None);
    btn.render(&mut renderer);
    assert_eq!(renderer.draws.len(), 0);
}

#[test]
fn test_button_focused_renders_arrow() {
    let mut renderer = TestRenderer::new();
    let mut btn = Button::new("X", (2, 3), RenderColor(255, 255, 255), None, None);
    btn.set_focused(true);
    btn.render(&mut renderer);
    // Should have 2 draws: '>' at (0, 3), 'X' at (2, 3)
    assert_eq!(renderer.draws.len(), 2);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '>' && cmd.pos == (0, 3))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'X' && cmd.pos == (2, 3))
    );
}

#[test]
fn test_button_click_fires_callbacks_map() {
    let callback_fired = Arc::new(Mutex::new(false));
    let cb = callback_fired.clone();
    let callback: WidgetCallback = Arc::new(move |_widget: &mut dyn UiWidget| {
        *cb.lock().unwrap() = true;
    });
    let mut btn = Button::new("Test", (0, 0), RenderColor(255, 255, 255), None, None);
    btn.set_callback("click", Some(callback));
    btn.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(*callback_fired.lock().unwrap());
}
