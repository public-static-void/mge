#[derive(Debug, Clone)]
pub enum UiEvent {
    Click { x: i32, y: i32 },
    KeyPress { key: String },
    // ... extend as needed
}
