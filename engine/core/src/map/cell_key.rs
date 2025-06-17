use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellKey {
    Square { x: i32, y: i32, z: i32 },
    Hex { q: i32, r: i32, z: i32 },
    Region { id: String },
}

impl CellKey {
    /// Convert a Position (as serde_json::Value) to a CellKey.
    pub fn from_position(position: &serde_json::Value) -> Option<Self> {
        let pos_obj = position.get("pos")?;
        if let Some(square) = pos_obj.get("Square") {
            Some(CellKey::Square {
                x: square.get("x")?.as_i64()? as i32,
                y: square.get("y")?.as_i64()? as i32,
                z: square.get("z")?.as_i64()? as i32,
            })
        } else if let Some(hex) = pos_obj.get("Hex") {
            Some(CellKey::Hex {
                q: hex.get("q")?.as_i64()? as i32,
                r: hex.get("r")?.as_i64()? as i32,
                z: hex.get("z")?.as_i64()? as i32,
            })
        } else if let Some(region) = pos_obj.get("Region") {
            Some(CellKey::Region {
                id: region.get("id")?.as_str()?.to_string(),
            })
        } else {
            None
        }
    }
}
