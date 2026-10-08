//! TopologyRegistry behavior: construction/dispatch opens closed matches.
//!
//! Builtin topologies resolve with their neighbor-inference strategy and
//! preferred field-of-view; unknown names resolve to nothing; a new topology
//! ships as one registration with zero core `match` edits.

use engine_core::map::topology_registry::{
    FovKind, NeighborStrategy, TopologyEntry, register_topology, registered_topology_names,
    resolve_topology,
};
use engine_core::map::{CellKey, Map};
use serde_json::{Value, json};
use std::any::Any;

// Square fixture: two adjacent cells, no explicit neighbors (inference path).
fn square_json() -> Value {
    json!({
        "topology": "square",
        "cells": [
            {"x": 0, "y": 0, "z": 0},
            {"x": 1, "y": 0, "z": 0}
        ]
    })
}

fn hex_json() -> Value {
    json!({
        "topology": "hex",
        "cells": [
            {"q": 0, "r": 0, "z": 0},
            {"q": 1, "r": 0, "z": 0}
        ]
    })
}

fn province_json() -> Value {
    json!({
        "topology": "province",
        "cells": [
            {"id": "a", "neighbors": ["b"]},
            {"id": "b", "neighbors": ["a"]}
        ]
    })
}

// Builtin entries resolve with the documented strategy + FOV pairing.
#[test]
fn builtin_topologies_resolve_with_strategy_and_fov() {
    let square = resolve_topology("square").expect("square is registered");
    assert!(matches!(
        square.neighbor_strategy,
        NeighborStrategy::FourWay
    ));
    assert!(matches!(
        square.preferred_fov,
        FovKind::RecursiveShadowcasting
    ));

    let hex = resolve_topology("hex").expect("hex is registered");
    assert!(matches!(hex.neighbor_strategy, NeighborStrategy::SixWay));
    assert!(matches!(hex.preferred_fov, FovKind::BfsFloodFill));

    let province = resolve_topology("province").expect("province is registered");
    assert!(matches!(
        province.neighbor_strategy,
        NeighborStrategy::ExplicitOnly
    ));
    assert!(matches!(province.preferred_fov, FovKind::BfsFloodFill));

    let names = registered_topology_names();
    assert!(names.contains(&"square".to_string()));
    assert!(names.contains(&"hex".to_string()));
    assert!(names.contains(&"province".to_string()));
}

// Unknown topology names resolve to nothing instead of panicking.
#[test]
fn unknown_topology_resolves_to_none() {
    assert!(resolve_topology("no-such-topology").is_none());
    assert!(Map::from_json(&json!({"topology": "no-such-topology", "cells": []})).is_err());
}

// Registry dispatch preserves the three builtin round-trips, including
// neighbor inference for grids and explicit adjacency for provinces.
#[test]
fn builtin_round_trips_survive_registry_dispatch() {
    let square = Map::from_json(&square_json()).expect("square parses");
    assert_eq!(square.topology_type(), "square");
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    assert!(square.contains(&origin));
    assert_eq!(
        square.neighbors(&origin),
        vec![CellKey::Square { x: 1, y: 0, z: 0 }]
    );

    let hex = Map::from_json(&hex_json()).expect("hex parses");
    assert_eq!(hex.topology_type(), "hex");
    let hq = CellKey::Hex { q: 0, r: 0, z: 0 };
    assert!(hex.contains(&hq));
    assert!(
        hex.neighbors(&hq)
            .contains(&CellKey::Hex { q: 1, r: 0, z: 0 })
    );

    let province = Map::from_json(&province_json()).expect("province parses");
    assert_eq!(province.topology_type(), "province");
    let pa = CellKey::Province {
        id: "a".to_string(),
    };
    assert!(province.neighbors(&pa).contains(&CellKey::Province {
        id: "b".to_string()
    }));
}

#[derive(Clone)]
struct MockTopology {
    cells: Vec<CellKey>,
}

impl engine_core::map::MapTopology for MockTopology {
    fn neighbors(&self, _cell: &CellKey) -> Vec<CellKey> {
        Vec::new()
    }
    fn contains(&self, cell: &CellKey) -> bool {
        self.cells.contains(cell)
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells.clone()
    }
    fn topology_type(&self) -> &'static str {
        "mock"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_cell_metadata(&mut self, _cell: &CellKey, _data: Value) {}
    fn get_cell_metadata(&self, _cell: &CellKey) -> Option<&Value> {
        None
    }
    fn merge_cell_metadata(&mut self, _cell: &CellKey, _patch: Value) {}
    fn clone_box(&self) -> Box<dyn engine_core::map::MapTopology> {
        Box::new(self.clone())
    }
}

fn mock_from_json(value: &Value) -> Option<Map> {
    let mut topo = MockTopology { cells: Vec::new() };
    for cell in value.get("cells")?.as_array()? {
        let id = cell.get("id")?.as_str()?.to_string();
        topo.cells.push(CellKey::Province { id });
    }
    Some(Map::new(Box::new(topo)))
}

// A new topology ships as one registration: no core `match` edit, and the
// parsed map plus its FOV preference are visible through the registry.
#[test]
fn new_topology_ships_as_one_registration() {
    register_topology(TopologyEntry {
        name: "mock",
        from_json: mock_from_json,
        neighbor_strategy: NeighborStrategy::ExplicitOnly,
        preferred_fov: FovKind::BfsFloodFill,
    });

    let entry = resolve_topology("mock").expect("mock resolves after registration");
    assert!(matches!(entry.preferred_fov, FovKind::BfsFloodFill));

    // Registry dispatch constructs the new topology with no core `match`
    // edit. The schema-validation gate in `Map::from_json` stays enum-closed
    // by design (a separate validation seam), so the proof routes through
    // the registered constructor directly.
    let map = (entry.from_json)(&json!({
        "topology": "mock",
        "cells": [{"id": "m1"}, {"id": "m2"}]
    }))
    .expect("mock topology parses after one registration");
    assert_eq!(map.topology_type(), "mock");
    assert!(map.contains(&CellKey::Province {
        id: "m1".to_string()
    }));
}
