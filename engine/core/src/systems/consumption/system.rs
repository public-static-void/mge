use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::trade::TRANSFER_EPSILON;
use serde_json::{Value as JsonValue, json};

/// One validated upkeep drain: `amount` of `kind` per tick.
struct Drain {
    kind: String,
    amount: f64,
}

/// Parse an `Upkeep` component into validated drains.
///
/// Entries with an empty kind or a non-finite/non-positive amount are skipped;
/// schema validation rejects those at `set_component` time, so this only
/// guards hand-built component data.
fn parse_drains(upkeep: &JsonValue) -> Vec<Drain> {
    upkeep
        .get("drains")
        .and_then(|v| v.as_array())
        .map(|drains| {
            drains
                .iter()
                .filter_map(|entry| {
                    let kind = entry.get("kind")?.as_str()?;
                    if kind.is_empty() {
                        return None;
                    }
                    let amount = entry.get("amount_per_tick")?.as_f64()?;
                    if !amount.is_finite() || amount <= 0.0 {
                        return None;
                    }
                    Some(Drain {
                        kind: kind.to_string(),
                        amount,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Read one resource balance from a stockpile value; a missing kind reads as 0.0.
fn stockpile_balance(stockpile: &JsonValue, kind: &str) -> f64 {
    stockpile
        .get("resources")
        .and_then(|r| r.get(kind))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

/// System: drains per-entity upkeep from each carrier's own stockpile once per tick.
///
/// Visits entities carrying `Upkeep` in ascending numeric entity-id order for
/// deterministic output. An entity without `Stockpile` is skipped with one
/// `consumption_shortage` event per drain kind (`available: 0.0`); an empty
/// drain list is a no-op. Runs in O(E x D) for E upkeep entities and D mean
/// drains per entity: direct component lookups, no world scan beyond upkeep
/// carriers.
pub struct ConsumptionSystem;

impl System for ConsumptionSystem {
    fn name(&self) -> &'static str {
        "ConsumptionSystem"
    }

    fn run(&mut self, world: &mut World) {
        let mut carriers = world.get_entities_with_component("Upkeep");
        carriers.sort_unstable();
        let turn = world.turn;

        for eid in carriers {
            let Some(upkeep) = world.get_component(eid, "Upkeep").cloned() else {
                continue;
            };
            let drains = parse_drains(&upkeep);
            if drains.is_empty() {
                continue;
            }

            let stockpile = world.get_component(eid, "Stockpile").cloned();
            let mut balances: Vec<(String, f64, f64)> = Vec::with_capacity(drains.len());
            for drain in &drains {
                let available = stockpile
                    .as_ref()
                    .map(|s| stockpile_balance(s, &drain.kind))
                    .unwrap_or(0.0);
                balances.push((drain.kind.clone(), drain.amount, available));
            }

            let short = balances
                .iter()
                .any(|(_, required, available)| *available + TRANSFER_EPSILON < *required);
            if short {
                for (kind, required, available) in &balances {
                    if *available + TRANSFER_EPSILON < *required {
                        let _ = world.send_event(
                            "consumption_shortage",
                            json!({
                                "type": "consumption_shortage",
                                "entity": eid,
                                "kind": kind,
                                "required": required,
                                "available": available,
                                "turn": turn,
                            }),
                        );
                    }
                }
                continue;
            }

            let mut updated = stockpile.expect("shortage-free entity carries a Stockpile");
            let resources = updated.get_mut("resources").and_then(|v| v.as_object_mut());
            let Some(map) = resources else { continue };
            for (kind, required, available) in &balances {
                let remainder = *available - *required;
                let floored = if remainder.abs() <= TRANSFER_EPSILON {
                    0.0
                } else {
                    remainder
                };
                map.insert(kind.clone(), json!(floored));
            }
            let _ = world.set_component(eid, "Stockpile", updated);
        }
    }
}
