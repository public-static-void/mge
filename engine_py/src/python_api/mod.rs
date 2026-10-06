//! Python API for the engine.
//!
//! This module contains the Python API for the engine, which is used to
//! create Python objects that can be used in Python scripts.

/// Body API
pub mod body;
/// Camera API
pub mod camera_api;
/// Component API
pub mod component;
/// Construction API (place_blueprint, get_construction_state, cancel_construction, demolish_building)
pub mod construction;
/// Crafting API (register_craft_recipe, list_craft_recipes, can_craft,
/// start_craft, get_craft_state, cancel_craft)
pub mod craft;
/// Death/decay API
pub mod death_decay;
/// Designer API (item definitions, equipment sets, loadouts)
pub mod designer;
/// Diplomacy API (get_relation, get_standing, modify_standing, declare_war,
/// declare_peace, propose_treaty, accept_treaty, break_treaty, list_treaties)
pub mod diplomacy;
/// Dungeon generation API
pub mod dungeon;
/// Economic API
pub mod economic;
/// Entity API
pub mod entity;
/// Equipment API
pub mod equipment;
/// Faction and Reputation API
pub mod faction;
/// Field-of-view API
pub mod fov;
/// Inventory API
pub mod inventory;
/// Job AI API
pub mod job_ai;
/// Job API
pub mod job_api;
/// Job board API
pub mod job_board;
/// Job children API
pub mod job_children;
/// Job dependencies API
pub mod job_dependencies;
/// Job events API
pub mod job_events;
/// Job production API
pub mod job_production;
/// Job query API
pub mod job_query;
/// Job reservation API
pub mod job_reservation;
/// Lore API (generate_founding_history, list_chronicle, get_chronicle_entry,
/// render_chronicle, chronicle_len, clear_lore_history)
pub mod lore;
/// Map API
pub mod map_api;
/// Material API
pub mod material;
/// Game mode API
pub mod mode;
/// Movement API
pub mod movement;
/// Multi-scale map navigation API
pub mod multiscale_map;
/// Narrative API (register_scenario, list_scenarios, get_scenario,
/// poll_pending_decisions, get_pending_decision, resolve_decision,
/// get_narrative_history)
pub mod narrative;
/// Noise and detection API
pub mod noise;
/// Region API
pub mod region;
/// Save/Load API
pub mod save_load;
/// Supply API (create_supply_link, remove_supply_link, list_supply_links,
/// set_supply_link_active, get_supply_link)
pub mod supply;
/// Tech Tree and Research API
pub mod tech_tree;
/// Temperature API
pub mod temperature;
/// Time API
pub mod time_of_day;
/// Trade API (transfer_stockpile_resource, has_active_trade_treaty,
/// execute_treaty_trade)
pub mod trade;
/// Turn API
pub mod turn;
/// UI API
pub mod ui;
/// Unit template API
pub mod unit_template;
/// Vehicle API (embark_vehicle, disembark_vehicle, assign_vehicle_path,
/// get_vehicle_occupants, is_mounted)
pub mod vehicle;
/// Weather API
pub mod weather;
/// World API
pub mod world;

pub use ui::UiApi;
pub use world::PyWorld;
