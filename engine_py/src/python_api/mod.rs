//! Python API for the engine.
//!
//! This module contains the Python API for the engine, which is used to
//! create Python objects that can be used in Python scripts.
//!
//! ## Bridge module registry
//!
//! The [`PY_API_MODULES`] list below is the Python bridge's shape-equivalent
//! of the Lua `ApiModule` registry (`engine_lua::lua_api::API_MODULES`) and
//! the WASM `ApiModule` registry (`engine_wasm::engine::API_MODULES`): one
//! entry per domain under a canonical `name`. The transport differs per
//! bridge by design — Lua populates a globals table, WASM populates a
//! linker, and here each domain is a trait (`XxxApi`) whose methods reach
//! scripts through the `#[pymethods]` delegation block on [`PyWorld`] in
//! `world.rs`. Each unit struct's doc comment names its trait and delegation
//! site, which is the `register` half of the protocol on this bridge. Adding
//! a domain is one trait (or free-function group) plus one unit struct here
//! plus one entry in [`PY_API_MODULES`]; only this inventory is shared,
//! never bridge-generation machinery.

use pyo3::prelude::*;

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

/// One Python scripting API domain.
///
/// Shape-equivalent to the Lua and WASM `ApiModule` protocols: a canonical
/// `name` plus one registration entry point. On this bridge registration is
/// the domain trait (or free-function group) plus its `#[pymethods]`
/// delegation on [`PyWorld`], so the trait carries only the shared half —
/// the name — while each implementor's doc comment records the home file
/// and delegation site that form the register half here.
pub trait PyApiModule: Sync {
    /// Canonical domain name, shared with the other bridges where the domain
    /// exists on all three (see `engine_core::api_registry::SHARED_API_DOMAINS`).
    fn name(&self) -> &'static str;
}

// Unit structs below adapt each domain's existing trait or free-function
// group to the `PyApiModule` protocol. The domain code keeps its focused
// shape; only the registry speaks the uniform inventory.

/// Body domain: `body::BodyApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct BodyModule;
impl PyApiModule for BodyModule {
    fn name(&self) -> &'static str {
        "body"
    }
}

/// Camera domain: `camera_api` free functions, delegated on `PyWorld` in `world.rs`.
pub struct CameraModule;
impl PyApiModule for CameraModule {
    fn name(&self) -> &'static str {
        "camera"
    }
}

/// Component domain: `component::ComponentApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct ComponentModule;
impl PyApiModule for ComponentModule {
    fn name(&self) -> &'static str {
        "component"
    }
}

/// Construction domain: `construction` free functions, delegated on `PyWorld` in `world.rs`.
pub struct ConstructionModule;
impl PyApiModule for ConstructionModule {
    fn name(&self) -> &'static str {
        "construction"
    }
}

/// Crafting domain: `craft` free functions, delegated on `PyWorld` in `world.rs`.
pub struct CraftModule;
impl PyApiModule for CraftModule {
    fn name(&self) -> &'static str {
        "craft"
    }
}

/// Death/decay domain: `death_decay::DeathDecayApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct DeathDecayModule;
impl PyApiModule for DeathDecayModule {
    fn name(&self) -> &'static str {
        "death_decay"
    }
}

/// Item/equipment designer domain (Python-bridge-local): `designer::DesignerApi`
/// trait, delegated on `PyWorld` in `world.rs`.
pub struct DesignerModule;
impl PyApiModule for DesignerModule {
    fn name(&self) -> &'static str {
        "designer"
    }
}

/// Diplomacy domain: `diplomacy` free functions, delegated on `PyWorld` in `world.rs`.
pub struct DiplomacyModule;
impl PyApiModule for DiplomacyModule {
    fn name(&self) -> &'static str {
        "diplomacy"
    }
}

/// Dungeon generation domain: `dungeon::DungeonApi`, delegated on `PyWorld` in `world.rs`.
pub struct DungeonModule;
impl PyApiModule for DungeonModule {
    fn name(&self) -> &'static str {
        "dungeon"
    }
}

/// Economic domain: `economic::EconomicApi` trait and helpers, delegated on `PyWorld` in `world.rs`.
pub struct EconomicModule;
impl PyApiModule for EconomicModule {
    fn name(&self) -> &'static str {
        "economic"
    }
}

/// Entity domain: `entity::EntityApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct EntityModule;
impl PyApiModule for EntityModule {
    fn name(&self) -> &'static str {
        "entity"
    }
}

/// Equipment domain: `equipment` trait and helpers, delegated on `PyWorld` in `world.rs`.
pub struct EquipmentModule;
impl PyApiModule for EquipmentModule {
    fn name(&self) -> &'static str {
        "equipment"
    }
}

/// Event bus domain: `crate::event_bus` free functions, delegated on
/// `PyWorld` in `world.rs`. Lives outside `python_api/` because the bus
/// state is a process-global transport detail, not world state.
pub struct EventBusModule;
impl PyApiModule for EventBusModule {
    fn name(&self) -> &'static str {
        "event_bus"
    }
}

/// Faction domain: `faction::FactionApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct FactionModule;
impl PyApiModule for FactionModule {
    fn name(&self) -> &'static str {
        "faction"
    }
}

/// Field-of-view domain: `fov::FovApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct FovModule;
impl PyApiModule for FovModule {
    fn name(&self) -> &'static str {
        "fov"
    }
}

/// Input domain: inline `get_user_input` on `PyWorld` in `world.rs`.
/// Thin by design — scripts read stdin directly; no input-provider plumbing
/// crosses this bridge.
pub struct InputModule;
impl PyApiModule for InputModule {
    fn name(&self) -> &'static str {
        "input"
    }
}

/// Inventory domain: `inventory` trait and helpers, delegated on `PyWorld` in `world.rs`.
pub struct InventoryModule;
impl PyApiModule for InventoryModule {
    fn name(&self) -> &'static str {
        "inventory"
    }
}

/// Job AI domain: `job_ai` free functions, delegated on `PyWorld` in `world.rs`.
pub struct JobAiModule;
impl PyApiModule for JobAiModule {
    fn name(&self) -> &'static str {
        "job_ai"
    }
}

/// Job board domain: `job_board` free functions, delegated on `PyWorld` in `world.rs`.
pub struct JobBoardModule;
impl PyApiModule for JobBoardModule {
    fn name(&self) -> &'static str {
        "job_board"
    }
}

/// Job cancel domain: `job_query::JobQueryApi::cancel_job`, delegated on
/// `PyWorld` in `world.rs`. Shares its home file with the query/mutation
/// domains; the registry keeps one entry per canonical name.
pub struct JobCancelModule;
impl PyApiModule for JobCancelModule {
    fn name(&self) -> &'static str {
        "job_cancel"
    }
}

/// Job children domain (Python-bridge-local): `job_children` free functions,
/// delegated on `PyWorld` in `world.rs`.
pub struct JobChildrenModule;
impl PyApiModule for JobChildrenModule {
    fn name(&self) -> &'static str {
        "job_children"
    }
}

/// Job dependencies domain (Python-bridge-local): `job_dependencies` free
/// functions, delegated on `PyWorld` in `world.rs`.
pub struct JobDependenciesModule;
impl PyApiModule for JobDependenciesModule {
    fn name(&self) -> &'static str {
        "job_dependencies"
    }
}

/// Job events domain: `job_events` free functions, delegated on `PyWorld` in `world.rs`.
pub struct JobEventsModule;
impl PyApiModule for JobEventsModule {
    fn name(&self) -> &'static str {
        "job_events"
    }
}

/// Job mutation domain: `job_query::JobQueryApi::{set_job_field, update_job}`,
/// delegated on `PyWorld` in `world.rs`. Shares its home file with the
/// query/cancel domains; the registry keeps one entry per canonical name.
pub struct JobMutationModule;
impl PyApiModule for JobMutationModule {
    fn name(&self) -> &'static str {
        "job_mutation"
    }
}

/// Production-job domain (Python-bridge-local): `job_production` free
/// functions, delegated on `PyWorld` in `world.rs`.
pub struct JobProductionModule;
impl PyApiModule for JobProductionModule {
    fn name(&self) -> &'static str {
        "job_production"
    }
}

/// Job query domain: `job_query::JobQueryApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct JobQueryModule;
impl PyApiModule for JobQueryModule {
    fn name(&self) -> &'static str {
        "job_query"
    }
}

/// Job reservation domain (Python-bridge-local): `job_reservation` free
/// functions, delegated on `PyWorld` in `world.rs`.
pub struct JobReservationModule;
impl PyApiModule for JobReservationModule {
    fn name(&self) -> &'static str {
        "job_reservation"
    }
}

/// Job system domain: `job_api` free functions (`assign_job`,
/// `register_job_type`, `advance_job_state`), delegated on `PyWorld` in
/// `world.rs`. Dispatches through `crate::job_bridge`, which stays a pure
/// callback-transport shim (see its module docs).
pub struct JobSystemModule;
impl PyApiModule for JobSystemModule {
    fn name(&self) -> &'static str {
        "job_system"
    }
}

/// Lore domain: `lore` free functions, delegated on `PyWorld` in `world.rs`.
pub struct LoreModule;
impl PyApiModule for LoreModule {
    fn name(&self) -> &'static str {
        "lore"
    }
}

/// Loot domain: inline loot-table methods on `PyWorld` in `world.rs` over
/// the world's `loot_tables` store.
pub struct LootModule;
impl PyApiModule for LootModule {
    fn name(&self) -> &'static str {
        "loot"
    }
}

/// Map domain: `map_api` free functions, delegated on `PyWorld` in `world.rs`.
pub struct MapModule;
impl PyApiModule for MapModule {
    fn name(&self) -> &'static str {
        "map"
    }
}

/// Material domain: `material::MaterialApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct MaterialModule;
impl PyApiModule for MaterialModule {
    fn name(&self) -> &'static str {
        "material"
    }
}

/// Game mode domain: `mode::ModeApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct ModeModule;
impl PyApiModule for ModeModule {
    fn name(&self) -> &'static str {
        "mode"
    }
}

/// Movement operations domain: `movement::MovementApi` trait, delegated on
/// `PyWorld` in `world.rs`.
pub struct MovementOpsModule;
impl PyApiModule for MovementOpsModule {
    fn name(&self) -> &'static str {
        "movement_ops"
    }
}

/// Multi-scale map domain: `multiscale_map` free functions, delegated on `PyWorld` in `world.rs`.
pub struct MultiscaleMapModule;
impl PyApiModule for MultiscaleMapModule {
    fn name(&self) -> &'static str {
        "multiscale_map"
    }
}

/// Narrative domain: `narrative` free functions, delegated on `PyWorld` in `world.rs`.
pub struct NarrativeModule;
impl PyApiModule for NarrativeModule {
    fn name(&self) -> &'static str {
        "narrative"
    }
}

/// Noise domain: `noise::NoiseApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct NoiseModule;
impl PyApiModule for NoiseModule {
    fn name(&self) -> &'static str {
        "noise"
    }
}

/// Region domain: `region::{RegionApi, ZoneApi}` traits, delegated on `PyWorld` in `world.rs`.
pub struct RegionModule;
impl PyApiModule for RegionModule {
    fn name(&self) -> &'static str {
        "region"
    }
}

/// Save/load domain: `save_load::SaveLoadApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct SaveLoadModule;
impl PyApiModule for SaveLoadModule {
    fn name(&self) -> &'static str {
        "save_load"
    }
}

/// Supply domain: `supply` free functions, delegated on `PyWorld` in `world.rs`.
pub struct SupplyModule;
impl PyApiModule for SupplyModule {
    fn name(&self) -> &'static str {
        "supply"
    }
}

/// System domain: `crate::system_bridge::SystemBridge` (`register_system` /
/// `run_system`), delegated on `PyWorld` in `world.rs`. Lives outside
/// `python_api/` because it stores interpreter callbacks — transport state,
/// not domain logic (see its module docs).
pub struct SystemModule;
impl PyApiModule for SystemModule {
    fn name(&self) -> &'static str {
        "system"
    }
}

/// Tech tree domain: `tech_tree::TechTreeApi` trait; `#[pymethods]`
/// delegation on `PyWorld` in `world.rs` calls through to the engine core
/// tech-tree helpers.
pub struct TechTreeModule;
impl PyApiModule for TechTreeModule {
    fn name(&self) -> &'static str {
        "tech_tree"
    }
}

/// Temperature domain: `temperature::TemperatureApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct TemperatureModule;
impl PyApiModule for TemperatureModule {
    fn name(&self) -> &'static str {
        "temperature"
    }
}

/// Time-of-day domain: `time_of_day::TimeOfDayApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct TimeOfDayModule;
impl PyApiModule for TimeOfDayModule {
    fn name(&self) -> &'static str {
        "time_of_day"
    }
}

/// Trade domain: `trade` free functions, delegated on `PyWorld` in `world.rs`.
pub struct TradeModule;
impl PyApiModule for TradeModule {
    fn name(&self) -> &'static str {
        "trade"
    }
}

/// Turn domain: `turn::TurnApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct TurnModule;
impl PyApiModule for TurnModule {
    fn name(&self) -> &'static str {
        "turn"
    }
}

/// UI domain: `ui::UiApi` class, registered on the module in `lib.rs`.
/// Unlike the world-bound domains above, this one is a standalone PyO3 class
/// rather than `PyWorld` methods — the register half on this bridge.
pub struct UiModule;
impl PyApiModule for UiModule {
    fn name(&self) -> &'static str {
        "ui"
    }
}

/// Unit template domain: `unit_template::UnitTemplateApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct UnitTemplateModule;
impl PyApiModule for UnitTemplateModule {
    fn name(&self) -> &'static str {
        "unit_template"
    }
}

/// Vehicle domain: `vehicle` free functions, delegated on `PyWorld` in `world.rs`.
pub struct VehicleModule;
impl PyApiModule for VehicleModule {
    fn name(&self) -> &'static str {
        "vehicle"
    }
}

/// Weather domain: `weather::WeatherApi` trait, delegated on `PyWorld` in `world.rs`.
pub struct WeatherModule;
impl PyApiModule for WeatherModule {
    fn name(&self) -> &'static str {
        "weather"
    }
}

/// World userdata domain (Python-bridge-local): the [`PyWorld`] class
/// itself, registered on the module in `lib.rs`.
pub struct WorldModule;
impl PyApiModule for WorldModule {
    fn name(&self) -> &'static str {
        "world"
    }
}

/// Worldgen domain: `crate::worldgen_bridge` plugin/validator/postprocessor
/// functions plus the map validator/postprocessor plumbing via `map_api`,
/// delegated on `PyWorld` in `world.rs`. Lives outside `python_api/`
/// because it adapts interpreter callbacks onto the global worldgen
/// registry — transport, not domain logic (see its module docs).
pub struct WorldgenModule;
impl PyApiModule for WorldgenModule {
    fn name(&self) -> &'static str {
        "worldgen"
    }
}

static BODY_MODULE: BodyModule = BodyModule;
static CAMERA_MODULE: CameraModule = CameraModule;
static COMPONENT_MODULE: ComponentModule = ComponentModule;
static CONSTRUCTION_MODULE: ConstructionModule = ConstructionModule;
static CRAFT_MODULE: CraftModule = CraftModule;
static DEATH_DECAY_MODULE: DeathDecayModule = DeathDecayModule;
static DESIGNER_MODULE: DesignerModule = DesignerModule;
static DIPLOMACY_MODULE: DiplomacyModule = DiplomacyModule;
static DUNGEON_MODULE: DungeonModule = DungeonModule;
static ECONOMIC_MODULE: EconomicModule = EconomicModule;
static ENTITY_MODULE: EntityModule = EntityModule;
static EQUIPMENT_MODULE: EquipmentModule = EquipmentModule;
static EVENT_BUS_MODULE: EventBusModule = EventBusModule;
static FACTION_MODULE: FactionModule = FactionModule;
static FOV_MODULE: FovModule = FovModule;
static INPUT_MODULE: InputModule = InputModule;
static INVENTORY_MODULE: InventoryModule = InventoryModule;
static JOB_AI_MODULE: JobAiModule = JobAiModule;
static JOB_BOARD_MODULE: JobBoardModule = JobBoardModule;
static JOB_CANCEL_MODULE: JobCancelModule = JobCancelModule;
static JOB_CHILDREN_MODULE: JobChildrenModule = JobChildrenModule;
static JOB_DEPENDENCIES_MODULE: JobDependenciesModule = JobDependenciesModule;
static JOB_EVENTS_MODULE: JobEventsModule = JobEventsModule;
static JOB_MUTATION_MODULE: JobMutationModule = JobMutationModule;
static JOB_PRODUCTION_MODULE: JobProductionModule = JobProductionModule;
static JOB_QUERY_MODULE: JobQueryModule = JobQueryModule;
static JOB_RESERVATION_MODULE: JobReservationModule = JobReservationModule;
static JOB_SYSTEM_MODULE: JobSystemModule = JobSystemModule;
static LORE_MODULE: LoreModule = LoreModule;
static LOOT_MODULE: LootModule = LootModule;
static MAP_MODULE: MapModule = MapModule;
static MATERIAL_MODULE: MaterialModule = MaterialModule;
static MODE_MODULE: ModeModule = ModeModule;
static MOVEMENT_OPS_MODULE: MovementOpsModule = MovementOpsModule;
static MULTISCALE_MAP_MODULE: MultiscaleMapModule = MultiscaleMapModule;
static NARRATIVE_MODULE: NarrativeModule = NarrativeModule;
static NOISE_MODULE: NoiseModule = NoiseModule;
static REGION_MODULE: RegionModule = RegionModule;
static SAVE_LOAD_MODULE: SaveLoadModule = SaveLoadModule;
static SUPPLY_MODULE: SupplyModule = SupplyModule;
static SYSTEM_MODULE: SystemModule = SystemModule;
static TECH_TREE_MODULE: TechTreeModule = TechTreeModule;
static TEMPERATURE_MODULE: TemperatureModule = TemperatureModule;
static TIME_OF_DAY_MODULE: TimeOfDayModule = TimeOfDayModule;
static TRADE_MODULE: TradeModule = TradeModule;
static TURN_MODULE: TurnModule = TurnModule;
static UI_MODULE: UiModule = UiModule;
static UNIT_TEMPLATE_MODULE: UnitTemplateModule = UnitTemplateModule;
static VEHICLE_MODULE: VehicleModule = VehicleModule;
static WEATHER_MODULE: WeatherModule = WeatherModule;
static WORLD_MODULE: WorldModule = WorldModule;
static WORLDGEN_MODULE: WorldgenModule = WorldgenModule;

/// The Python bridge API-module registry: one entry per domain, alphabetical.
/// Adding a domain appends one entry here; the pytest pins this order, so
/// removals and renames fail loudly instead of drifting silently.
pub static PY_API_MODULES: &[&dyn PyApiModule] = &[
    &BODY_MODULE,
    &CAMERA_MODULE,
    &COMPONENT_MODULE,
    &CONSTRUCTION_MODULE,
    &CRAFT_MODULE,
    &DEATH_DECAY_MODULE,
    &DESIGNER_MODULE,
    &DIPLOMACY_MODULE,
    &DUNGEON_MODULE,
    &ECONOMIC_MODULE,
    &ENTITY_MODULE,
    &EQUIPMENT_MODULE,
    &EVENT_BUS_MODULE,
    &FACTION_MODULE,
    &FOV_MODULE,
    &INPUT_MODULE,
    &INVENTORY_MODULE,
    &JOB_AI_MODULE,
    &JOB_BOARD_MODULE,
    &JOB_CANCEL_MODULE,
    &JOB_CHILDREN_MODULE,
    &JOB_DEPENDENCIES_MODULE,
    &JOB_EVENTS_MODULE,
    &JOB_MUTATION_MODULE,
    &JOB_PRODUCTION_MODULE,
    &JOB_QUERY_MODULE,
    &JOB_RESERVATION_MODULE,
    &JOB_SYSTEM_MODULE,
    &LORE_MODULE,
    &LOOT_MODULE,
    &MAP_MODULE,
    &MATERIAL_MODULE,
    &MODE_MODULE,
    &MOVEMENT_OPS_MODULE,
    &MULTISCALE_MAP_MODULE,
    &NARRATIVE_MODULE,
    &NOISE_MODULE,
    &REGION_MODULE,
    &SAVE_LOAD_MODULE,
    &SUPPLY_MODULE,
    &SYSTEM_MODULE,
    &TECH_TREE_MODULE,
    &TEMPERATURE_MODULE,
    &TIME_OF_DAY_MODULE,
    &TRADE_MODULE,
    &TURN_MODULE,
    &UI_MODULE,
    &UNIT_TEMPLATE_MODULE,
    &VEHICLE_MODULE,
    &WEATHER_MODULE,
    &WORLD_MODULE,
    &WORLDGEN_MODULE,
];

/// Returns the canonical names of the given modules, in order.
pub fn py_api_module_names(modules: &[&dyn PyApiModule]) -> Vec<&'static str> {
    modules.iter().map(|m| m.name()).collect()
}

/// Introspection entry point for the registry: returns every registered
/// domain name, in registry order. Pinned by
/// `engine_py/tests/test_api_module_registry.py`.
#[pyfunction]
pub fn list_api_modules() -> Vec<String> {
    py_api_module_names(PY_API_MODULES)
        .into_iter()
        .map(str::to_string)
        .collect()
}
