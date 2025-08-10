/// UI events
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Mouse click
    Click {
        /// The x coordinate of the click
        x: i32,
        /// The y coordinate of the click
        y: i32,
    },
    /// Key press
    KeyPress {
        /// The key pressed
        key: String,
    },
}
