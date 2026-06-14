use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Marker component for camera entities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Camera;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_roundtrip() {
        let original = Camera;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Camera = serde_json::from_str(&json).unwrap();
        // Verify round-trip succeeded for a unit struct
        let _ = (original, deserialized);
    }
}
