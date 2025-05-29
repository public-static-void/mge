use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Marker component for camera entities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Camera;
