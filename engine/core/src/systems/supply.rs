//! Per-tick supply push between linked stockpiles.
//!
//! Each tick [`SupplySystem`] visits entities carrying `SupplyLink` in
//! ascending numeric entity-id order for deterministic output, delivering
//! `min(amount_per_tick, capacity_per_tick)` of `kind` from source to target
//! through [`crate::trade::transfer_stockpile_resource`]. Inactive links are
//! skipped silently; endpoints missing `Stockpile` emit one `supply_blocked`
//! event with reason `missing_stockpile`; cross-faction legs at war emit one
//! `supply_blocked` event with reason `war`; positioned endpoints with no
//! route between their cells emit one `supply_blocked` event with reason
//! `no_path` (endpoints off every known map stay unblocked); source shortfalls emit one
//! `supply_shortfall` event with no mutation; successful legs emit one
//! `supply_delivered` event. Link ids are collected up front and each leg is
//! itself atomic, so legs compose in sorted order with no mid-iteration link
//! mutation. Runs in O(L): one link-entity iteration, O(1) work per link.
//! No RNG anywhere on this path.

use crate::diplomacy::{RelationState, get_relation};
use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::faction::get_faction;
use crate::map::CellKey;
use crate::supply::parse_supply_link;
use crate::trade::{TransferError, transfer_stockpile_resource};
use serde_json::json;

/// System: pushes capped resource amounts along supply links once per tick.
pub struct SupplySystem;

impl System for SupplySystem {
    fn name(&self) -> &'static str {
        "SupplySystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["ConsumptionSystem"]
    }

    fn run(&mut self, world: &mut World) {
        let mut links = world.get_entities_with_component("SupplyLink");
        links.sort_unstable();
        let turn = world.turn;

        for link in links {
            let Some(record) = world.get_component(link, "SupplyLink").cloned() else {
                continue;
            };
            let Some(route) = parse_supply_link(&record) else {
                continue;
            };
            if !route.active {
                continue;
            }
            if !world.has_component(route.source, "Stockpile")
                || !world.has_component(route.target, "Stockpile")
            {
                let _ = world.send_event(
                    "supply_blocked",
                    json!({
                        "type": "supply_blocked",
                        "link": link,
                        "reason": "missing_stockpile",
                        "turn": turn,
                    }),
                );
                continue;
            }
            if let (Some(source_faction), Some(target_faction)) = (
                get_faction(world, route.source),
                get_faction(world, route.target),
            ) && source_faction != target_faction
                && get_relation(world, &source_faction, &target_faction) == RelationState::War
            {
                let _ = world.send_event(
                    "supply_blocked",
                    json!({
                        "type": "supply_blocked",
                        "link": link,
                        "reason": "war",
                        "turn": turn,
                    }),
                );
                continue;
            }
            let source_cell = world
                .get_component(route.source, "Position")
                .and_then(CellKey::from_position);
            let target_cell = world
                .get_component(route.target, "Position")
                .and_then(CellKey::from_position);
            if let (Some(start), Some(goal)) = (source_cell, target_cell)
                && let Some(map) = world.get_map()
                && map.contains(&start)
                && map.contains(&goal)
                && map.find_path(&start, &goal).is_none()
            {
                let _ = world.send_event(
                    "supply_blocked",
                    json!({
                        "type": "supply_blocked",
                        "link": link,
                        "reason": "no_path",
                        "turn": turn,
                    }),
                );
                continue;
            }
            let request = route.amount_per_tick.min(route.capacity_per_tick);
            match transfer_stockpile_resource(
                world,
                route.source,
                route.target,
                &route.kind,
                request,
            ) {
                Ok(()) => {
                    let _ = world.send_event(
                        "supply_delivered",
                        json!({
                            "type": "supply_delivered",
                            "link": link,
                            "kind": route.kind,
                            "amount": request,
                            "turn": turn,
                        }),
                    );
                }
                Err(TransferError::InsufficientFunds { available, .. }) => {
                    let _ = world.send_event(
                        "supply_shortfall",
                        json!({
                            "type": "supply_shortfall",
                            "link": link,
                            "kind": route.kind,
                            "requested": request,
                            "available": available,
                            "turn": turn,
                        }),
                    );
                }
                Err(TransferError::NoStockpile(_)) => {
                    let _ = world.send_event(
                        "supply_blocked",
                        json!({
                            "type": "supply_blocked",
                            "link": link,
                            "reason": "missing_stockpile",
                            "turn": turn,
                        }),
                    );
                }
                Err(_) => {
                    continue;
                }
            }
        }
    }
}
