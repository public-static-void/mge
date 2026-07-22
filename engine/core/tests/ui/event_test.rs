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


