use serde::Deserialize;

/// Represents a resource amount
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceAmount {
    /// The kind of the resource
    pub kind: String,
    /// The amount
    pub amount: i64,
}

/// Tool requirement: an item that must be present, optionally consumed at start.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolReq {
    /// Item id that must be present on the crafter
    pub item: String,
    /// Whether the tool is consumed at `start_craft`
    #[serde(default)]
    pub consumed: bool,
}

/// Material requirement: quantity consumed from the crafter's stockpile.
#[derive(Debug, Clone, Deserialize)]
pub struct MaterialAmount {
    /// Material key consumed from `Stockpile.resources`
    pub material: String,
    /// Amount consumed
    pub amount: i64,
}

/// Skill gate for a recipe.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillReq {
    /// Skill name; defaults to "crafting" when omitted
    #[serde(default = "default_craft_skill")]
    pub skill: String,
    /// Minimum skill level required
    pub level: i64,
}

fn default_craft_skill() -> String {
    "crafting".to_string()
}

/// Item-entity output descriptor. When `None`, the recipe is stockpile-only.
#[derive(Debug, Clone, Deserialize)]
pub struct OutputItem {
    /// Item id carried by the spawned entity's `Item` component
    pub id: String,
    /// Display name for the spawned `Item` component
    pub name: String,
    /// Equipment slot for the spawned `Item` component
    pub slot: String,
}

/// Represents a recipe
#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    /// The name of the recipe
    pub name: String,
    /// The input resources
    #[serde(default)]
    pub inputs: Vec<ResourceAmount>,
    /// The output resources
    #[serde(default)]
    pub outputs: Vec<ResourceAmount>,
    ///The duration
    pub duration: i64,
    /// Tool requirements; empty means no tool gate
    #[serde(default)]
    pub tools: Vec<ToolReq>,
    /// Material requirements consumed from stockpile at start
    #[serde(default)]
    pub materials: Vec<MaterialAmount>,
    /// Skill gate; None means ungated
    #[serde(default)]
    pub required_skill: Option<SkillReq>,
    /// Item-entity output; None means stockpile-only (legacy behavior)
    #[serde(default)]
    pub output_item: Option<OutputItem>,
    /// Reserved workbench affinity; ignored by logic
    #[serde(default)]
    pub station: Option<String>,
    /// Deterministic XP override; falls back to registry `base_xp`
    #[serde(default)]
    pub xp: Option<i64>,
}
