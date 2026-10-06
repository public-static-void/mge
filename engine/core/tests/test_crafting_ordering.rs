//! Crafting execution-ordering pin.
//!
//! `SYSTEM_EXECUTION_ORDER` places `ConsumptionSystem` immediately after
//! `EconomicSystem`, `SupplySystem` immediately after consumption, and
//! `CraftingSystem` immediately after supply (and therefore before
//! `ConstructionSystem`), so upkeep drains observe the same-tick production
//! results, supply pushes observe post-upkeep stockpiles, and craft progress
//! observes post-supply stockpiles, and `order_systems` preserves that
//! relative order.

// Crafting reads the post-supply stockpile state each tick, so upkeep drains
// run back-to-back with the economic loop, supply pushes follow consumption,
// and crafting follows supply before construction consumes outputs.
use engine_core::systems::{SYSTEM_EXECUTION_ORDER, order_systems};

#[test]
fn test_crafting_pinned_immediately_after_economic() {
    let order: Vec<&str> = SYSTEM_EXECUTION_ORDER.to_vec();
    let economic = order
        .iter()
        .position(|name| *name == "EconomicSystem")
        .expect("EconomicSystem is ordered");
    let consumption = order
        .iter()
        .position(|name| *name == "ConsumptionSystem")
        .expect("ConsumptionSystem is ordered");
    let supply = order
        .iter()
        .position(|name| *name == "SupplySystem")
        .expect("SupplySystem is ordered");
    let crafting = order
        .iter()
        .position(|name| *name == "CraftingSystem")
        .expect("CraftingSystem is ordered");
    let construction = order
        .iter()
        .position(|name| *name == "ConstructionSystem")
        .expect("ConstructionSystem is ordered");
    assert_eq!(consumption, economic + 1);
    assert_eq!(supply, consumption + 1);
    assert_eq!(crafting, supply + 1);
    assert_eq!(construction, crafting + 1);

    let names = vec![
        "ConstructionSystem".to_string(),
        "CraftingSystem".to_string(),
        "SupplySystem".to_string(),
        "ConsumptionSystem".to_string(),
        "EconomicSystem".to_string(),
    ];
    assert_eq!(
        order_systems(&names),
        vec![
            "EconomicSystem".to_string(),
            "ConsumptionSystem".to_string(),
            "SupplySystem".to_string(),
            "CraftingSystem".to_string(),
            "ConstructionSystem".to_string(),
        ]
    );
}
