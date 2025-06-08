use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobType {
    pub name: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub effects: Vec<JobEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobEffect {
    pub action: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}
