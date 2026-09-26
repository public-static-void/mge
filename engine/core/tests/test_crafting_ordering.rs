//! Crafting execution-ordering pin.
//!
//! `SYSTEM_EXECUTION_ORDER` places `CraftingSystem` immediately after
//! `EconomicSystem` (and therefore before `ConstructionSystem`), so craft
//! progress observes the same-tick production results, and `order_systems`
//! preserves that relative order.

use engine_core::systems::{SYSTEM_EXECUTION_ORDER, order_systems};

// Crafting reads the post-production stockpile state each tick, so it runs
// back-to-back with the economic loop before construction consumes outputs.
#[test]
fn test_crafting_pinned_immediately_after_economic() {
    let order: Vec<&str> = SYSTEM_EXECUTION_ORDER.to_vec();
    let economic = order
        .iter()
        .position(|name| *name == "EconomicSystem")
        .expect("EconomicSystem is ordered");
    let crafting = order
        .iter()
        .position(|name| *name == "CraftingSystem")
        .expect("CraftingSystem is ordered");
    let construction = order
        .iter()
        .position(|name| *name == "ConstructionSystem")
        .expect("ConstructionSystem is ordered");
    assert_eq!(crafting, economic + 1);
    assert_eq!(construction, crafting + 1);

    let names = vec![
        "ConstructionSystem".to_string(),
        "CraftingSystem".to_string(),
        "EconomicSystem".to_string(),
    ];
    assert_eq!(
        order_systems(&names),
        vec![
            "EconomicSystem".to_string(),
            "CraftingSystem".to_string(),
            "ConstructionSystem".to_string(),
        ]
    );
}
