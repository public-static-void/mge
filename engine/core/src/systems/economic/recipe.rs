use serde::Deserialize;

/// Represents a resource amount
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceAmount {
    /// The kind of the resource
    pub kind: String,
    /// The amount
    pub amount: i64,
}

/// Represents a recipe
#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    /// The name of the recipe
    pub name: String,
    /// The input resources
    pub inputs: Vec<ResourceAmount>,
    /// The output resources
    pub outputs: Vec<ResourceAmount>,
    ///The duration
    pub duration: i64,
}
