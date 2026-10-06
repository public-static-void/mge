//! Supply-link lifecycle: validated per-tick pushes between stockpiles.
//!
//! A link is an entity carrying a `SupplyLink` component
//! (`source`, `target`, `kind`, `amount_per_tick`, `capacity_per_tick`,
//! `active`). [`create_supply_link`] validates before spawning, so every
//! `Err` leaves no link entity behind. Per-tick execution lives in
//! [`crate::systems::supply::SupplySystem`], which moves
//! `min(amount_per_tick, capacity_per_tick)` via
//! [`crate::trade::transfer_stockpile_resource`].

use crate::ecs::world::World;
use serde_json::json;

/// One validated supply route between two stockpile entities.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyLink {
    /// Entity supplying the resource.
    pub source: u32,
    /// Entity receiving the resource.
    pub target: u32,
    /// Resource kind moved each tick.
    pub kind: String,
    /// Requested push per tick before the capacity cap.
    pub amount_per_tick: f64,
    /// Per-tick ceiling; delivery is `min(amount, capacity)`.
    pub capacity_per_tick: f64,
    /// Inactive links are skipped silently by the tick.
    pub active: bool,
}

/// Failure modes for [`create_supply_link`].
#[derive(Debug, Clone, PartialEq)]
pub enum SupplyError {
    /// Source and target are the same entity.
    SameEndpoint,
    /// The resource kind is empty.
    UnknownKind(String),
    /// An amount is zero, negative, NaN, or infinite.
    NonPositiveAmount,
    /// An endpoint carries no `Stockpile` component (holds the entity).
    NoStockpile(u32),
    /// No `SupplyLink` component exists on the entity (holds the entity).
    UnknownLink(u32),
    /// The validated record could not be stored (holds the store error).
    Store(String),
}

impl std::fmt::Display for SupplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupplyError::SameEndpoint => {
                write!(f, "supply link source and target must differ")
            }
            SupplyError::UnknownKind(kind) => {
                write!(f, "unknown resource kind '{kind}'")
            }
            SupplyError::NonPositiveAmount => {
                write!(f, "supply amounts must be positive and finite")
            }
            SupplyError::NoStockpile(entity) => {
                write!(f, "entity {entity} has no Stockpile component")
            }
            SupplyError::UnknownLink(entity) => {
                write!(f, "entity {entity} has no SupplyLink component")
            }
            SupplyError::Store(inner) => {
                write!(f, "supply link store failed: {inner}")
            }
        }
    }
}

impl std::error::Error for SupplyError {}

/// Parses a stored `SupplyLink` value; `None` on any malformed shape.
///
/// Guards hand-built component data; schema validation rejects bad shapes at
/// `set_component` time on the creation path.
pub fn parse_supply_link(value: &serde_json::Value) -> Option<SupplyLink> {
    let source = value.get("source")?.as_u64()? as u32;
    let target = value.get("target")?.as_u64()? as u32;
    let kind = value.get("kind")?.as_str()?;
    if kind.is_empty() {
        return None;
    }
    let amount_per_tick = value.get("amount_per_tick")?.as_f64()?;
    let capacity_per_tick = value.get("capacity_per_tick")?.as_f64()?;
    if !amount_per_tick.is_finite()
        || !capacity_per_tick.is_finite()
        || amount_per_tick <= 0.0
        || capacity_per_tick <= 0.0
    {
        return None;
    }
    let active = value.get("active")?.as_bool()?;
    Some(SupplyLink {
        source,
        target,
        kind: kind.to_string(),
        amount_per_tick,
        capacity_per_tick,
        active,
    })
}

/// Creates a supply link after validating endpoints and amounts.
///
/// Rejects self-links, empty kinds, non-positive/non-finite amounts, and
/// endpoints without `Stockpile`. Validation precedes spawning, so failures
/// leave no link entity behind. New links start active.
pub fn create_supply_link(
    world: &mut World,
    source: u32,
    target: u32,
    kind: &str,
    amount_per_tick: f64,
    capacity_per_tick: f64,
) -> Result<u32, SupplyError> {
    if source == target {
        return Err(SupplyError::SameEndpoint);
    }
    if kind.is_empty() {
        return Err(SupplyError::UnknownKind(kind.to_string()));
    }
    if !amount_per_tick.is_finite()
        || !capacity_per_tick.is_finite()
        || amount_per_tick <= 0.0
        || capacity_per_tick <= 0.0
    {
        return Err(SupplyError::NonPositiveAmount);
    }
    if !world.has_component(source, "Stockpile") {
        return Err(SupplyError::NoStockpile(source));
    }
    if !world.has_component(target, "Stockpile") {
        return Err(SupplyError::NoStockpile(target));
    }
    let link = world.spawn_entity();
    world
        .set_component(
            link,
            "SupplyLink",
            json!({
                "source": source,
                "target": target,
                "kind": kind,
                "amount_per_tick": amount_per_tick,
                "capacity_per_tick": capacity_per_tick,
                "active": true,
            }),
        )
        .map_err(SupplyError::Store)?;
    Ok(link)
}

/// Reads a link record; `None` when the entity carries no `SupplyLink`.
pub fn get_supply_link(world: &World, link: u32) -> Option<SupplyLink> {
    world
        .get_component(link, "SupplyLink")
        .and_then(parse_supply_link)
}

/// Removes a link entity's `SupplyLink` component.
///
/// Errors on unknown links; the link entity itself stays despawnable by the
/// caller. Removing the last component of a bare link leaves an empty entity.
pub fn remove_supply_link(world: &mut World, link: u32) -> Result<(), SupplyError> {
    if !world.has_component(link, "SupplyLink") {
        return Err(SupplyError::UnknownLink(link));
    }
    world
        .remove_component(link, "SupplyLink")
        .map_err(|_| SupplyError::UnknownLink(link))
}

/// Lists all link entity ids in ascending order.
pub fn list_supply_links(world: &World) -> Vec<u32> {
    let mut links = world.get_entities_with_component("SupplyLink");
    links.sort_unstable();
    links
}

/// Toggles a link's `active` flag; errors on unknown links.
pub fn set_supply_link_active(
    world: &mut World,
    link: u32,
    active: bool,
) -> Result<(), SupplyError> {
    let mut record = world
        .get_component(link, "SupplyLink")
        .cloned()
        .ok_or(SupplyError::UnknownLink(link))?;
    record["active"] = json!(active);
    world
        .set_component(link, "SupplyLink", record)
        .map_err(SupplyError::Store)
}
