//! Vehicle execution-ordering pin.
//!
//! Covers the M3 slice: `SYSTEM_EXECUTION_ORDER` places `MovementSystem`
//! immediately followed by `VehicleSystem` after `EcosystemSystem` and before
//! `ProcessDeaths`, and `order_systems` preserves that relative order.

use engine_core::systems::{SYSTEM_EXECUTION_ORDER, order_systems};

// Movement and vehicle carriers run back-to-back after the ecosystem tick so
// mounted riders end the tick co-located with their vehicle, before death
// processing observes final positions.
#[test]
fn test_movement_then_vehicle_pinned_between_ecosystem_and_deaths() {
    let order: Vec<&str> = SYSTEM_EXECUTION_ORDER.to_vec();
    let ecosystem = order
        .iter()
        .position(|name| *name == "EcosystemSystem")
        .expect("EcosystemSystem is ordered");
    let movement = order
        .iter()
        .position(|name| *name == "MovementSystem")
        .expect("MovementSystem is ordered");
    let vehicle = order
        .iter()
        .position(|name| *name == "VehicleSystem")
        .expect("VehicleSystem is ordered");
    let deaths = order
        .iter()
        .position(|name| *name == "ProcessDeaths")
        .expect("ProcessDeaths is ordered");
    assert_eq!(movement, ecosystem + 1);
    assert_eq!(vehicle, movement + 1);
    assert_eq!(deaths, vehicle + 1);

    let names = vec![
        "ProcessDeaths".to_string(),
        "VehicleSystem".to_string(),
        "EcosystemSystem".to_string(),
        "MovementSystem".to_string(),
    ];
    assert_eq!(
        order_systems(&names),
        vec![
            "EcosystemSystem".to_string(),
            "MovementSystem".to_string(),
            "VehicleSystem".to_string(),
            "ProcessDeaths".to_string(),
        ]
    );
}
