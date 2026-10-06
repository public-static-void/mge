//! Supply-link operations for [`WasmWorld`](super::wasm::WasmWorld).
//!
//! Thin mirror of [`crate::supply`]: validation runs before spawning, so
//! every `Err` leaves no link entity behind. New links start active. Error
//! strings prefix the core variant name so guest-side assertions match on
//! it, exactly like the script bridges and
//! [`trade_ops`](super::trade_ops::WasmWorld::transfer_stockpile_resource).

use super::wasm::WasmWorld;
use crate::supply::{SupplyLink, parse_supply_link};

impl WasmWorld {
    /// Reads one link's stored record; `None` when the entity carries no
    /// well-formed `SupplyLink` component.
    fn supply_record(&self, link: u32) -> Option<SupplyLink> {
        self.components
            .get("SupplyLink")
            .and_then(|map| map.get(&link))
            .and_then(parse_supply_link)
    }

    /// Creates a supply link after validating endpoints and amounts.
    ///
    /// Mirrors [`crate::supply::create_supply_link`]: rejects self-links,
    /// empty kinds, non-positive/non-finite amounts, and endpoints without
    /// `Stockpile`. Validation precedes spawning, so failures leave no link
    /// entity behind. New links start active.
    pub fn create_supply_link(
        &mut self,
        source: u32,
        target: u32,
        kind: &str,
        amount_per_tick: f64,
        capacity_per_tick: f64,
    ) -> Result<u32, String> {
        if source == target {
            return Err("SameEndpoint: supply link source and target must differ".to_string());
        }
        if kind.is_empty() {
            return Err(format!("UnknownKind: unknown resource kind '{kind}'"));
        }
        if !amount_per_tick.is_finite()
            || !capacity_per_tick.is_finite()
            || amount_per_tick <= 0.0
            || capacity_per_tick <= 0.0
        {
            return Err(
                "NonPositiveAmount: supply amounts must be positive and finite".to_string(),
            );
        }
        let stocked = |world: &Self, entity: u32| {
            world
                .components
                .get("Stockpile")
                .is_some_and(|map| map.contains_key(&entity))
        };
        if !stocked(self, source) {
            return Err(format!(
                "NoStockpile: entity {source} has no Stockpile component"
            ));
        }
        if !stocked(self, target) {
            return Err(format!(
                "NoStockpile: entity {target} has no Stockpile component"
            ));
        }
        let link = self.spawn_entity();
        let record = serde_json::json!({
            "source": source,
            "target": target,
            "kind": kind,
            "amount_per_tick": amount_per_tick,
            "capacity_per_tick": capacity_per_tick,
            "active": true,
        });
        self.components
            .entry("SupplyLink".to_string())
            .or_default()
            .insert(link, record);
        Ok(link)
    }

    /// Reads a link record; `None` on unknown links.
    pub fn get_supply_link(&self, link: u32) -> Option<SupplyLink> {
        self.supply_record(link)
    }

    /// Removes a link entity's `SupplyLink` component.
    ///
    /// Errors on unknown links; the link entity itself stays for the caller
    /// to despawn, matching [`crate::supply::remove_supply_link`].
    pub fn remove_supply_link(&mut self, link: u32) -> Result<(), String> {
        let known = self
            .components
            .get("SupplyLink")
            .is_some_and(|map| map.contains_key(&link));
        if !known {
            return Err(format!(
                "UnknownLink: entity {link} has no SupplyLink component"
            ));
        }
        self.remove_component(link, "SupplyLink")
            .map_err(|_| format!("UnknownLink: entity {link} has no SupplyLink component"))
    }

    /// Lists all link entity ids in ascending order.
    pub fn list_supply_links(&self) -> Vec<u32> {
        let mut links: Vec<u32> = self
            .components
            .get("SupplyLink")
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default();
        links.sort_unstable();
        links
    }

    /// Toggles a link's `active` flag; errors on unknown links.
    pub fn set_supply_link_active(&mut self, link: u32, active: bool) -> Result<(), String> {
        let mut record = self
            .components
            .get("SupplyLink")
            .and_then(|map| map.get(&link))
            .cloned()
            .ok_or_else(|| format!("UnknownLink: entity {link} has no SupplyLink component"))?;
        if let Some(obj) = record.as_object_mut() {
            obj.insert("active".to_string(), serde_json::json!(active));
        }
        self.components
            .entry("SupplyLink".to_string())
            .or_default()
            .insert(link, record);
        Ok(())
    }
}
