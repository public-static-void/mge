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

        let mut rng = rand::thread_rng();

        // Sum all weights for weighted selection
        let total_weight: u64 = table.entries.iter().map(|e| e.weight as u64).sum();

        if total_weight == 0 {
            return Err(LootError::EmptyTable(name.to_string()));
        }

        // Weighted random selection: pick one entry proportional to its weight
        let pick = rand::Rng::gen_range(&mut rng, 0u64..total_weight);
        let mut cumulative = 0u64;

        for entry in &table.entries {
            cumulative += entry.weight as u64;
            if pick < cumulative {
                let count = if entry.min_count == entry.max_count {
                    entry.min_count
                } else {
                    rand::Rng::gen_range(&mut rng, entry.min_count..=entry.max_count)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_roll() {
        let mut registry = LootTableRegistry::new();
        registry
            .define_table(
                "test",
                vec![LootEntry {
                    item_id: "item1".into(),
                    weight: 100,
                    min_count: 1,
                    max_count: 1,
                }],
            )
            .unwrap();

        let result = registry.roll("test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "item1");
        assert_eq!(result[0].1, 1);
    }

    #[test]
    fn test_empty_table_returns_error() {
        let mut registry = LootTableRegistry::new();
        registry.define_table("empty", vec![]).unwrap();
        let result = registry.roll("empty");
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_table_returns_error() {
        let registry = LootTableRegistry::new();
        let result = registry.roll("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_weight_validated() {
        let mut registry = LootTableRegistry::new();
        let result = registry.define_table(
            "bad",
            vec![LootEntry {
                item_id: "item".into(),
                weight: 0,
                min_count: 1,
                max_count: 1,
            }],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_min_max_count_range() {
        let mut registry = LootTableRegistry::new();
        registry
            .define_table(
                "multi",
                vec![LootEntry {
                    item_id: "coins".into(),
                    weight: 100,
                    min_count: 2,
                    max_count: 5,
                }],
            )
            .unwrap();

        for _ in 0..20 {
            let result = registry.roll("multi").unwrap();
            assert_eq!(result.len(), 1);
            assert!(result[0].1 >= 2 && result[0].1 <= 5);
        }
    }

    #[test]
    fn test_weighted_distribution() {
        let mut registry = LootTableRegistry::new();
        registry
            .define_table(
                "weighted",
                vec![
                    LootEntry {
                        item_id: "common".into(),
                        weight: 90,
                        min_count: 1,
                        max_count: 1,
                    },
                    LootEntry {
                        item_id: "rare".into(),
                        weight: 10,
                        min_count: 1,
                        max_count: 1,
                    },
                ],
            )
            .unwrap();

        let mut common_count = 0u32;
        let mut rare_count = 0u32;
        let total_rolls = 100;
        for _ in 0..total_rolls {
            let result = registry.roll("weighted").unwrap();
            assert_eq!(result.len(), 1, "weighted-sum should return exactly 1 item");
            if result[0].0 == "common" {
                common_count += 1;
            } else {
                rare_count += 1;
            }
        }
        assert_eq!(
            common_count + rare_count,
            total_rolls,
            "every roll should produce exactly one item"
        );
        assert!(
            common_count > rare_count,
            "common (weight 90) should be selected more often than rare (weight 10): had {} vs {}",
            common_count,
            rare_count
        );
    }

    #[test]
    fn test_has_table() {
        let mut registry = LootTableRegistry::new();
        assert!(!registry.has_table("foo"));
        registry
            .define_table(
                "foo",
                vec![LootEntry {
                    item_id: "bar".into(),
                    weight: 100,
                    min_count: 1,
                    max_count: 1,
                }],
            )
            .unwrap();
        assert!(registry.has_table("foo"));
    }

    #[test]
    fn test_table_names() {
        let mut registry = LootTableRegistry::new();
        registry.define_table("a", vec![]).unwrap();
        registry.define_table("b", vec![]).unwrap();
        let names = registry.table_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    #[test]
    fn test_remove_table() {
        let mut registry = LootTableRegistry::new();
        registry
            .define_table(
                "temp",
                vec![LootEntry {
                    item_id: "x".into(),
                    weight: 100,
                    min_count: 1,
                    max_count: 1,
                }],
            )
            .unwrap();
        assert!(registry.has_table("temp"));
        registry.remove_table("temp");
        assert!(!registry.has_table("temp"));
        assert!(registry.roll("temp").is_err());
    }

    #[test]
    fn test_invalid_min_max_count() {
        let mut registry = LootTableRegistry::new();
        let result = registry.define_table(
            "bad",
            vec![LootEntry {
                item_id: "x".into(),
                weight: 100,
                min_count: 5,
                max_count: 1,
            }],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_define_overwrites() {
        let mut registry = LootTableRegistry::new();
        registry
            .define_table(
                "dupe",
                vec![LootEntry {
                    item_id: "old".into(),
                    weight: 100,
                    min_count: 1,
                    max_count: 1,
                }],
            )
            .unwrap();
        registry
            .define_table(
                "dupe",
                vec![LootEntry {
                    item_id: "new".into(),
                    weight: 100,
                    min_count: 1,
                    max_count: 1,
                }],
            )
            .unwrap();
        let result = registry.roll("dupe").unwrap();
        assert_eq!(result[0].0, "new");
    }
}
