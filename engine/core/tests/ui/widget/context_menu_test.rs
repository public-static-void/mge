use engine_core::presentation::ui::widget::context_menu::{ContextMenu, ContextMenuEntry};
use engine_core::presentation::ui::UiEvent;
use engine_core::presentation::renderer::{RenderColor, TestRenderer};
use std::sync::{Arc, Mutex};

fn make_entry(label: &str) -> ContextMenuEntry {
    ContextMenuEntry::new(label, true, None)
}

fn make_disabled_entry(label: &str) -> ContextMenuEntry {
    ContextMenuEntry::new(label, false, None)
}

#[test]
fn test_context_menu_show_sets_visible() {
    let entries = vec![make_entry("A")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    assert!(!menu.visible);
    menu.show((5, 5));
    assert!(menu.visible);
    assert_eq!(menu.pos, (5, 5));
    assert_eq!(menu.selected, 0);
}

#[test]
fn test_context_menu_hide_clears_visible() {
    let entries = vec![make_entry("A")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    assert!(menu.visible);
    menu.hide();
    assert!(!menu.visible);
}

#[test]
fn test_context_menu_keyboard_down_navigates() {
    let entries = vec![make_entry("A"), make_entry("B"), make_entry("C")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    assert_eq!(menu.selected, 0);
    menu.handle_event(&UiEvent::KeyPress {
        key: "Down".to_string(),
    });
    assert_eq!(menu.selected, 1);
    menu.handle_event(&UiEvent::KeyPress {
        key: "Down".to_string(),
    });
    assert_eq!(menu.selected, 2);
}

#[test]
fn test_context_menu_keyboard_down_wraps_to_first() {
    let entries = vec![make_entry("A"), make_entry("B")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    menu.selected = 1;
    menu.handle_event(&UiEvent::KeyPress {
        key: "Down".to_string(),
    });
    assert_eq!(menu.selected, 0); // wraps to first
}

#[test]
fn test_context_menu_keyboard_up_navigates() {
    let entries = vec![make_entry("A"), make_entry("B"), make_entry("C")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    menu.selected = 2;
    menu.handle_event(&UiEvent::KeyPress {
        key: "Up".to_string(),
    });
    assert_eq!(menu.selected, 1);
    menu.handle_event(&UiEvent::KeyPress {
        key: "Up".to_string(),
    });
    assert_eq!(menu.selected, 0);
}

#[test]
fn test_context_menu_keyboard_up_wraps_to_last() {
    let entries = vec![make_entry("A"), make_entry("B")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    menu.selected = 0;
    menu.handle_event(&UiEvent::KeyPress {
        key: "Up".to_string(),
    });
    assert_eq!(menu.selected, 1); // wraps to last
}

#[test]
fn test_context_menu_enter_selects_and_fires_action() {
    let action_fired = Arc::new(Mutex::new(false));
    let action = action_fired.clone();
    let entries = vec![ContextMenuEntry::new(
        "Action",
        true,
        Some(Box::new(move || *action.lock().unwrap() = true)),
    )];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    menu.handle_event(&UiEvent::KeyPress {
        key: "Enter".to_string(),
    });
    assert!(*action_fired.lock().unwrap());
    assert!(!menu.visible); // menu hides after action
}

#[test]
fn test_context_menu_esc_closes() {
    let entries = vec![make_entry("A")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    assert!(menu.visible);
    menu.handle_event(&UiEvent::KeyPress {
        key: "Esc".to_string(),
    });
    assert!(!menu.visible);
}

#[test]
fn test_context_menu_empty_entries_does_not_crash() {
    let entries = vec![];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    assert!(menu.visible);
    // Esc closes the menu (no keyboard nav to avoid div-by-zero in production)
    menu.handle_event(&UiEvent::KeyPress {
        key: "Esc".to_string(),
    });
    assert!(!menu.visible);
}

#[test]
fn test_context_menu_click_outside_closes() {
    let entries = vec![make_entry("A")];
    let mut menu = ContextMenu::new(
        (10, 10),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((10, 10));
    assert!(menu.visible);
    menu.handle_event(&UiEvent::Click { x: 0, y: 0 });
    assert!(!menu.visible);
}

#[test]
fn test_context_menu_skip_disabled_on_down() {
    let entries = vec![make_disabled_entry("Disabled"), make_entry("Enabled")];
    let mut menu = ContextMenu::new(
        (0, 0),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((0, 0));
    menu.selected = 0;
    menu.handle_event(&UiEvent::KeyPress {
        key: "Down".to_string(),
    });
    assert_eq!(menu.selected, 1); // skip disabled and go to enabled
}

#[test]
fn test_context_menu_render_visible() {
    let mut renderer = TestRenderer::new();
    let entries = vec![make_entry("Alpha")];
    let mut menu = ContextMenu::new(
        (5, 5),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.show((5, 5));
    menu.render(&mut renderer);
    // Should render background and label
    assert!(
        renderer
            .draws
            .iter()
            .any(|cmd| cmd.glyph == 'A' && cmd.pos == (6, 5))
    );
}

#[test]
fn test_context_menu_render_hidden_is_noop() {
    let mut renderer = TestRenderer::new();
    let entries = vec![make_entry("Alpha")];
    let mut menu = ContextMenu::new(
        (5, 5),
        entries,
        RenderColor(255, 255, 255),
        RenderColor(0, 0, 0),
    );
    menu.render(&mut renderer); // not shown
    assert_eq!(renderer.draws.len(), 0);
}
