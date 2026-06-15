use engine_core::presentation::ui::widget::dropdown::Dropdown;
use engine_core::presentation::ui::UiEvent;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use std::sync::{Arc, Mutex};

#[test]
fn test_dropdown_expands_on_header_click() {
    let options = vec!["A".to_string(), "B".to_string()];
    let mut dd = Dropdown::new((0, 0), 10, options, RenderColor(255, 255, 255));
    assert!(!dd.expanded);
    dd.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(dd.expanded);
}

#[test]
fn test_dropdown_select_item_fires_callback() {
    let options = vec!["One".to_string(), "Two".to_string()];
    let selected = Arc::new(Mutex::new(None));
    let sel_cb = selected.clone();
    let mut dd = Dropdown::new((10, 5), 10, options, RenderColor(255, 255, 255));
    dd.set_on_select(Box::new(move |val| *sel_cb.lock().unwrap() = Some(val)));

    // Expand
    dd.handle_event(&UiEvent::Click { x: 10, y: 5 });
    assert!(dd.expanded);

    // Click on "Two" which is at y = 5 + 1 + 1 = 7
    dd.handle_event(&UiEvent::Click { x: 10, y: 7 });
    assert_eq!(dd.selected, Some("Two".to_string()));
    assert_eq!(*selected.lock().unwrap(), Some("Two".to_string()));
    assert!(!dd.expanded);
}

#[test]
fn test_dropdown_toggle_header_twice() {
    // Current implementation: clicking header toggles expanded state.
    // Click-outside does NOT collapse the dropdown.
    let options = vec!["A".to_string(), "B".to_string()];
    let mut dd = Dropdown::new((10, 5), 10, options, RenderColor(255, 255, 255));
    dd.handle_event(&UiEvent::Click { x: 10, y: 5 }); // expand
    assert!(dd.expanded);
    dd.handle_event(&UiEvent::Click { x: 10, y: 5 }); // collapse by clicking header again
    assert!(!dd.expanded);
}

#[test]
fn test_dropdown_empty_items_does_not_crash() {
    let options: Vec<String> = vec![];
    let mut dd = Dropdown::new((0, 0), 10, options, RenderColor(255, 255, 255));
    // Expand empty dropdown — no crash
    dd.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(dd.expanded);
    // Click outside on empty expanded dropdown — no crash, stays expanded
    dd.handle_event(&UiEvent::Click { x: 100, y: 100 });
    assert!(dd.expanded); // click-outside does not collapse
    // Click header again to collapse
    dd.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!dd.expanded);
}

#[test]
fn test_dropdown_render_header() {
    let mut renderer = TestRenderer::new();
    let options = vec!["Option".to_string()];
    let mut dd = Dropdown::new((5, 3), 10, options, RenderColor(0, 255, 0));
    dd.render(&mut renderer);
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '<' && cmd.pos == (5, 3))
    );
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == '>' && cmd.pos == (14, 3))
    );
}

#[test]
fn test_dropdown_render_expanded_shows_options() {
    let mut renderer = TestRenderer::new();
    let options = vec!["X".to_string(), "Y".to_string()];
    let mut dd = Dropdown::new((0, 0), 10, options, RenderColor(255, 255, 255));
    dd.expanded = true;
    dd.render(&mut renderer);
    // Options should be rendered below header
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
            .any(|cmd| cmd.glyph == 'Y' && cmd.pos == (0, 2))
    );
}
