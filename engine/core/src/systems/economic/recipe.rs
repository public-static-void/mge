use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceAmount {
    pub kind: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub inputs: Vec<ResourceAmount>,
    pub outputs: Vec<ResourceAmount>,
    pub duration: i64,
}
