use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellKey {
    Square { x: i32, y: i32, z: i32 },
    Hex { q: i32, r: i32, z: i32 },
    Region { id: String },
}

impl CellKey {
    /// Convert a Position (as serde_json::Value) to a CellKey.
    /// Handles:
    /// - { "pos": { "Square": ... } }
    /// - { "Square": ... }
    /// - { "pos": { "Hex": ... } }
    /// - { "Hex": ... }
    /// - { "pos": { "Region": ... } }
    /// - { "Region": ... }
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
        } else if let Some(region) = obj.get("Region") {
            Some(CellKey::Region {
                id: region.get("id")?.as_str()?.to_string(),
            })
        } else {
            None
        }
    }
}
