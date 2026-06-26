use engine_core::loot::{LootEntry, LootTableRegistry};

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
