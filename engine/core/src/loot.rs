use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A weighted entry in a loot table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    /// Item identifier string.
    pub item_id: String,
    /// Relative probability weight (0 means never selected).
    pub weight: u32,
    /// Minimum quantity to drop on selection.
    #[serde(default = "default_count")]
    pub min_count: u32,
    /// Maximum quantity to drop on selection.
    #[serde(default = "default_count")]
    pub max_count: u32,
}

fn default_count() -> u32 {
    1
}

/// A named collection of loot entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootTable {
    /// Name of this loot table.
    pub name: String,
    /// Entries defining possible loot drops.
    pub entries: Vec<LootEntry>,
}

/// Errors from loot table operations.
#[derive(Debug)]
pub enum LootError {
    /// Roll on a table that was never defined.
    TableNotFound(String),
    /// Roll on a table with zero entries or all zero-weight entries.
    EmptyTable(String),
    /// Entry validation failure at define time.
    InvalidEntry(String),
}

impl std::fmt::Display for LootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LootError::TableNotFound(name) => {
                write!(f, "loot table '{}' not found", name)
            }
            LootError::EmptyTable(name) => {
                write!(f, "loot table '{}' has no entries", name)
            }
            LootError::InvalidEntry(msg) => {
                write!(f, "invalid loot entry: {}", msg)
            }
        }
    }
}

impl std::error::Error for LootError {}

/// Registry holding all defined loot tables.
///
/// Tables are defined at runtime via `define_table()` and are not serialized
/// (the `World` field is marked `#[serde(skip)]`).
#[derive(Debug, Default)]
pub struct LootTableRegistry {
    tables: HashMap<String, LootTable>,
}

impl LootTableRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    /// Define or replace a named loot table.
    ///
    /// Validates that each entry has:
    /// - `weight > 0`
    /// - `min_count > 0` and `max_count > 0`
    /// - `min_count <= max_count`
    pub fn define_table(&mut self, name: &str, entries: Vec<LootEntry>) -> Result<(), LootError> {
        for entry in &entries {
            if entry.weight == 0 {
                return Err(LootError::InvalidEntry(format!(
                    "entry '{}' has zero weight",
                    entry.item_id
                )));
            }
            if entry.min_count == 0 || entry.max_count == 0 {
                return Err(LootError::InvalidEntry(format!(
                    "entry '{}' has zero count",
                    entry.item_id
                )));
            }
            if entry.min_count > entry.max_count {
                return Err(LootError::InvalidEntry(format!(
                    "entry '{}' has min_count > max_count",
                    entry.item_id
                )));
            }
        }
        self.tables.insert(
            name.to_string(),
            LootTable {
                name: name.to_string(),
                entries,
            },
        );
        Ok(())
    }

    /// Roll on a named loot table using weighted-sum selection.
    ///
    /// Sums all entry weights, picks a random number in `[0, total_weight)`,
    /// and returns the entry whose cumulative weight range contains the pick.
    /// Exactly one entry is returned per roll call (with its randomized count).
    ///
    /// Returns `Err(LootError::TableNotFound)` if the table was never defined,
    /// or `Err(LootError::EmptyTable)` if the table has no entries or all
    /// entries have zero weight.
    pub fn roll(&self, name: &str) -> Result<Vec<(String, u32)>, LootError> {
        let table = self
            .tables
            .get(name)
            .ok_or_else(|| LootError::TableNotFound(name.to_string()))?;

        if table.entries.is_empty() {
            return Err(LootError::EmptyTable(name.to_string()));
        }

        let mut rng = rand::rng();

        // Sum all weights for weighted selection
        let total_weight: u64 = table.entries.iter().map(|e| e.weight as u64).sum();

        if total_weight == 0 {
            return Err(LootError::EmptyTable(name.to_string()));
        }

        // Weighted random selection: pick one entry proportional to its weight
        let pick = rng.random_range(0u64..total_weight);
        let mut cumulative = 0u64;

        for entry in &table.entries {
            cumulative += entry.weight as u64;
            if pick < cumulative {
                let count = if entry.min_count == entry.max_count {
                    entry.min_count
                } else {
                    rng.random_range(entry.min_count..=entry.max_count)
                };
                return Ok(vec![(entry.item_id.clone(), count)]);
            }
        }

        // Fallback — should never reach here with valid entries
        Err(LootError::EmptyTable(name.to_string()))
    }

    /// Check if a table with the given name exists.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// Return the names of all defined tables.
    pub fn table_names(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Remove a table from the registry.
    pub fn remove_table(&mut self, name: &str) {
        self.tables.remove(name);
    }
}
