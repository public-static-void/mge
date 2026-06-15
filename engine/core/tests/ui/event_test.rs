use engine_core::presentation::ui::UiEvent;

#[test]
fn test_click_event_creation() {
    let event = UiEvent::Click { x: 10, y: 20 };
    match event {
        UiEvent::Click { x, y } => {
            assert_eq!(x, 10);
            assert_eq!(y, 20);
        }
        _ => panic!("Expected Click event"),
    }
}

#[test]
fn test_keypress_event_creation() {
    let event = UiEvent::KeyPress {
        key: "Enter".to_string(),
    };
    match event {
        UiEvent::KeyPress { key } => {
            assert_eq!(key, "Enter");
        }
        _ => panic!("Expected KeyPress event"),
    }
}

#[test]
fn test_click_event_clone() {
    let event = UiEvent::Click { x: 5, y: 3 };
    let cloned = event.clone();
    match cloned {
        UiEvent::Click { x, y } => {
            assert_eq!(x, 5);
            assert_eq!(y, 3);
        }
        _ => panic!("Expected Click event"),
    }
}

#[test]
fn test_keypress_event_debug_format() {
    let event = UiEvent::KeyPress {
        key: "Space".to_string(),
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("KeyPress"));
    assert!(debug_str.contains("Space"));
}
