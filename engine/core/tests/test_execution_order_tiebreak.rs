//! Dependency-aware execution ordering tie-break.
//!
//! The closed `SYSTEM_EXECUTION_ORDER` pack stays the default; systems absent
//! from the list fall back to topological order over `System::dependencies()`,
//! with registration order breaking remaining ties.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::systems::{order_systems, order_systems_with_dependencies};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn deps(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(name, ds)| (name.to_string(), ds.iter().map(|d| d.to_string()).collect()))
        .collect()
}

// Unlisted systems honor declared dependencies even when registration order
// runs against them; the listed pack still leads.
#[test]
fn unlisted_systems_follow_declared_dependencies() {
    let names = vec![
        "TiebreakBeta".to_string(),
        "EconomicSystem".to_string(),
        "TiebreakAlpha".to_string(),
    ];
    let dependencies = deps(&[("TiebreakBeta", &["TiebreakAlpha"])]);
    let ordered = order_systems_with_dependencies(&names, &dependencies);
    let pos = |n: &str| ordered.iter().position(|x| x == n).unwrap();
    assert!(
        pos("EconomicSystem") < pos("TiebreakAlpha"),
        "listed pack stays first: {ordered:?}"
    );
    assert!(
        pos("TiebreakAlpha") < pos("TiebreakBeta"),
        "dependency orders first despite later registration: {ordered:?}"
    );
}

// Remaining ties break by registration order, and unknown dependency names
// are ignored instead of failing.
#[test]
fn ties_break_by_registration_order_and_unknown_deps_ignored() {
    let names = vec!["TiebreakOne".to_string(), "TiebreakTwo".to_string()];
    let ordered = order_systems_with_dependencies(&names, &deps(&[]));
    assert_eq!(ordered, names);

    let chained = vec!["TiebreakTwo".to_string(), "TiebreakOne".to_string()];
    let with_unknown = deps(&[("TiebreakOne", &["MissingSystem"])]);
    let ordered = order_systems_with_dependencies(&chained, &with_unknown);
    assert_eq!(ordered, chained);
}

// The legacy entry point keeps its contract: listed pack first, remainder in
// input order.
#[test]
fn legacy_ordering_keeps_pack_first_and_input_order() {
    let names = vec![
        "TiebreakBeta".to_string(),
        "EconomicSystem".to_string(),
        "TiebreakAlpha".to_string(),
    ];
    assert_eq!(
        order_systems(&names),
        vec![
            "EconomicSystem".to_string(),
            "TiebreakBeta".to_string(),
            "TiebreakAlpha".to_string(),
        ]
    );
}

struct OrderProbe {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl System for OrderProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run(&mut self, _world: &mut World) {
        self.log.lock().unwrap().push(self.name.to_string());
    }
    fn dependencies(&self) -> &'static [&'static str] {
        if self.name == "TiebreakBeta" {
            &["TiebreakAlpha"]
        } else {
            &[]
        }
    }
}

// End to end: a new unlisted system with a declared dependency ticks after
// its dependency without editing the closed order list — even when it was
// registered first.
#[test]
fn simulation_tick_honors_unlisted_dependency() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut world = make_test_world();
    world.register_system(OrderProbe {
        name: "TiebreakBeta",
        log: Arc::clone(&log),
    });
    world.register_system(OrderProbe {
        name: "TiebreakAlpha",
        log: Arc::clone(&log),
    });
    let world_rc = Rc::new(RefCell::new(world));
    World::simulation_tick(Rc::clone(&world_rc));

    let log = log.lock().unwrap();
    let pos = |n: &str| log.iter().position(|x| x == n).unwrap();
    assert!(
        pos("TiebreakAlpha") < pos("TiebreakBeta"),
        "tick runs dependency first: {log:?}"
    );
}
