//! Shared bridge-domain contract for the scripting API-module registries.
//!
//! The Lua (`engine_lua`) and WASM (`engine_wasm`) bridges expose the same
//! engine domains under one canonical name each. This list is the single
//! source of truth for the domains that must exist on both bridges. Each
//! bridge pins its full module list in its own registry test and asserts it
//! is a superset of this shared set, so neither list can silently drift.
//! Bridge-only domains (Lua `world`, WASM `designer`, ...) live only in the
//! owning bridge's list.

/// Canonical domain names every scripting bridge must expose.
///
/// Entries are alphabetical. A domain is added here only once both bridges
/// expose it under this exact name.
pub const SHARED_API_DOMAINS: &[&str] = &[
    "body",
    "camera",
    "component",
    "construction",
    "craft",
    "death_decay",
    "diplomacy",
    "dungeon",
    "economic",
    "entity",
    "equipment",
    "event_bus",
    "faction",
    "fov",
    "input",
    "inventory",
    "job_ai",
    "job_board",
    "job_cancel",
    "job_events",
    "job_mutation",
    "job_query",
    "job_system",
    "lore",
    "loot",
    "map",
    "material",
    "mode",
    "movement_ops",
    "multiscale_map",
    "narrative",
    "noise",
    "region",
    "save_load",
    "supply",
    "system",
    "tech_tree",
    "temperature",
    "time_of_day",
    "trade",
    "turn",
    "ui",
    "unit_template",
    "vehicle",
    "weather",
    "worldgen",
];
