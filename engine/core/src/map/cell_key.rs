use serde::{Deserialize, Serialize};

/// Represents a key for a cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellKey {
    /// Represents a key for a square cell.
    Square {
        /// The x coordinate of the cell.
        x: i32,
        /// The y coordinate of the cell.
        y: i32,
        /// The z coordinate of the cell.
        z: i32,
    },
    /// Represents a key for a hex cell.
    Hex {
        /// The q coordinate of the cell.
        q: i32,
        /// The r coordinate of the cell.
        r: i32,
        /// The z coordinate of the cell.
        z: i32,
    },
    /// Represents a key for a province cell.
    Province {
        /// The ID of the province.
        id: String,
    },
}

impl CellKey {
    /// Convert a Position (as serde_json::Value) to a CellKey.
    /// Handles:
    /// - { "pos": { "Square": ... } }
    /// - { "Square": ... }
    /// - { "pos": { "Hex": ... } }
    /// - { "Hex": ... }
    /// - { "pos": { "Province": ... } }
    /// - { "Province": ... }
    pub fn from_position(position: &serde_json::Value) -> Option<Self> {
        // Try to unwrap "pos" if present
        let obj = if let Some(pos) = position.get("pos") {
            pos
        } else {
            position
        };

        if let Some(square) = obj.get("Square") {
            Some(CellKey::Square {
                x: square.get("x")?.as_i64()? as i32,
                y: square.get("y")?.as_i64()? as i32,
                z: square.get("z")?.as_i64()? as i32,
            })
        } else if let Some(hex) = obj.get("Hex") {
            Some(CellKey::Hex {
                q: hex.get("q")?.as_i64()? as i32,
                r: hex.get("r")?.as_i64()? as i32,
                z: hex.get("z")?.as_i64()? as i32,
            })
        } else if let Some(province) = obj.get("Province") {
            Some(CellKey::Province {
                id: province.get("id")?.as_str()?.to_string(),
            })
        } else {
            None
        }
    }
}
