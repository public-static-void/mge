use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents one of the four seasons, derived from day count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    /// Spring (days 0-29, 120-149, ...)
    Spring,
    /// Summer (days 30-59, 150-179, ...)
    Summer,
    /// Autumn (days 60-89, 180-209, ...)
    Autumn,
    /// Winter (days 90-119, 210-239, ...)
    Winter,
}

impl Season {
    /// Returns the Season for a given day count.
    /// Uses 30-day seasons, 120-day year.
    pub fn from_day(day: u64) -> Self {
        match (day % 120) / 30 {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Season::Spring => write!(f, "spring"),
            Season::Summer => write!(f, "summer"),
            Season::Autumn => write!(f, "autumn"),
            Season::Winter => write!(f, "winter"),
        }
    }
}
