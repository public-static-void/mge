//! Input abstraction for the presentation layer.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEvent {
    KeyPress(char),
    Quit,
    // Extend as needed (mouse, etc.)
}

pub trait PresentationInput {
    /// Poll for the next input event (blocking or non-blocking as desired).
    fn poll_event(&mut self) -> Option<InputEvent>;
}
