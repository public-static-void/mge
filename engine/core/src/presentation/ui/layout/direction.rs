use serde::{Deserialize, Serialize};

/// A layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutDirection {
    /// Horizontal layout.
    Row,
    /// Vertical layout.
    Column,
}

/// A layout alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    /// Start alignment.
    Start,
    /// Center alignment.
    Center,
    /// End alignment.
    End,
}

/// A layout padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Padding {
    /// The left padding.
    pub left: i32,
    /// The right padding.
    pub right: i32,
    /// The top padding.
    pub top: i32,
    /// The bottom padding.
    pub bottom: i32,
}

impl Padding {
    /// Create a uniform padding.
    pub fn uniform(pad: i32) -> Self {
        Self {
            left: pad,
            right: pad,
            top: pad,
            bottom: pad,
        }
    }
}
