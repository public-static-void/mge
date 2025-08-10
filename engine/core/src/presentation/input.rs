//! Input abstraction for the presentation layer.

/// An input event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// A key was pressed.
    KeyPress(char),
    /// Quit the application.
    Quit,
}

/// A presentation layer input source.
pub trait PresentationInput {
    /// Poll for the next input event (blocking or non-blocking as desired).
    fn poll_event(&mut self) -> Option<InputEvent>;
}
