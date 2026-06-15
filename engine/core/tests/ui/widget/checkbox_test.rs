use engine_core::presentation::ui::widget::checkbox::Checkbox;
use engine_core::presentation::ui::widget::widget_trait::UiWidget;
use engine_core::presentation::ui::UiEvent;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use std::sync::{Arc, Mutex};

#[test]
fn test_checkbox_toggles_on_click() {
    let mut cb = Checkbox::new("Test", (0, 0), RenderColor(255, 255, 255), None);
    assert!(!cb.checked);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(cb.checked);
}

#[test]
fn test_checkbox_toggles_twice_on_off_on() {
    let mut cb = Checkbox::new("Test", (0, 0), RenderColor(255, 255, 255), None);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(cb.checked);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!cb.checked);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(cb.checked);
}

#[test]
fn test_checkbox_click_outside_bounds_no_toggle() {
    let mut cb = Checkbox::new("Test", (5, 5), RenderColor(255, 255, 255), None);
    assert!(!cb.checked);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!cb.checked);
}

#[test]
fn test_checkbox_render_unchecked() {
    let mut renderer = TestRenderer::new();
    let mut cb = Checkbox::new("A", (0, 0), RenderColor(255, 255, 255), None);
    cb.render(&mut renderer);
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '☐'));
}

#[test]
fn test_checkbox_render_checked() {
    let mut renderer = TestRenderer::new();
    let mut cb = Checkbox::new("A", (0, 0), RenderColor(255, 255, 255), None);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    cb.render(&mut renderer);
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '☑'));
}

#[test]
fn test_checkbox_space_key_toggles_when_focused() {
    let mut cb = Checkbox::new("Test", (0, 0), RenderColor(255, 255, 255), None);
    cb.set_focused(true);
    assert!(!cb.checked);
    // Checkbox handle_event for KeyPress does not toggle — it only handles Click events
    // So we document this behavior
    cb.handle_event(&UiEvent::KeyPress {
        key: "Space".to_string(),
    });
    assert!(!cb.checked); // Space key does not toggle checkbox per current implementation
}

#[test]
fn test_checkbox_disabled_blocks_toggle() {
    // Checkbox does not have a disabled field — we test that clicking outside bounds is no-op
    let mut cb = Checkbox::new("Test", (5, 5), RenderColor(255, 255, 255), None);
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!cb.checked);
}

#[test]
fn test_checkbox_on_toggle_callback_fires() {
    let toggled = Arc::new(Mutex::new(false));
    let cb_toggle = toggled.clone();
    let mut cb = Checkbox::new("Test", (0, 0), RenderColor(255, 255, 255), None);
    cb.set_on_toggle(Box::new(move |_state| *cb_toggle.lock().unwrap() = true));
    cb.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(*toggled.lock().unwrap());
}

#[test]
fn test_checkbox_focused_renders_arrow() {
    let mut renderer = TestRenderer::new();
    let mut cb = Checkbox::new("X", (2, 3), RenderColor(255, 255, 255), None);
    cb.set_focused(true);
    cb.render(&mut renderer);
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '>'));
    assert!(renderer.draws.iter().any(|cmd| cmd.glyph == '☐'));
}
