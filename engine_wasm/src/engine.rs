use crate::host_api::body::register_body_api;
use crate::host_api::body_part_damage::register_body_part_damage_api;
use crate::host_api::camera::register_camera_api;
use crate::host_api::component::register_component_api;
use crate::host_api::construction::register_construction_api;
use crate::host_api::craft::register_craft_api;
use crate::host_api::death_decay::register_death_decay_api;
use crate::host_api::designer::register_designer_api;
use crate::host_api::diplomacy::register_diplomacy_api;
use crate::host_api::dungeon::register_dungeon_api;
use crate::host_api::economic::register_economic_api;
use crate::host_api::entity::register_entity_api;
use crate::host_api::equipment::register_equipment_api;
use crate::host_api::event_bus::register_event_bus_api;
use crate::host_api::faction::register_faction_api;
use crate::host_api::fov::register_fov_api;
use crate::host_api::input::register_input_api;
use crate::host_api::inventory::register_inventory_api;
use crate::host_api::job_ai::register_job_ai_api;
use crate::host_api::job_board::register_job_board_api;
use crate::host_api::job_cancel::register_job_cancel_api;
use crate::host_api::job_events::register_job_events_api;
use crate::host_api::job_mutation::register_job_mutation_api;
use crate::host_api::job_query::register_job_query_api;
use crate::host_api::job_system::register_job_system_api;
use crate::host_api::loot::register_loot_api;
use crate::host_api::lore::register_lore_api;
use crate::host_api::map::register_map_api;
use crate::host_api::material::register_material_api;
use crate::host_api::mode::register_mode_api;
use crate::host_api::movement_ops::register_movement_ops_api;
use crate::host_api::multiscale_map::register_multiscale_map_api;
use crate::host_api::narrative::register_narrative_api;
use crate::host_api::noise::register_noise_api;
use crate::host_api::region::register_region_api;
use crate::host_api::save_load::register_save_load_api;
use crate::host_api::supply::register_supply_api;
use crate::host_api::system::register_system_api;
use crate::host_api::tech_tree::register_tech_tree_api;
use crate::host_api::temperature::register_temperature_api;
use crate::host_api::time_of_day::register_time_of_day_api;
use crate::host_api::trade::register_trade_api;
use crate::host_api::turn::register_turn_api;
use crate::host_api::ui::register_ui_api;
use crate::host_api::ui_events::register_ui_events_api;
use crate::host_api::ui_tree::register_ui_tree_api;
use crate::host_api::unit_template::register_unit_template_api;
use crate::host_api::vehicle::register_vehicle_api;
use crate::host_api::weather::register_weather_api;
use crate::host_api::world_userdata::register_world_userdata_api;
use crate::host_api::worldgen::register_worldgen_api;
use anyhow::Result;
use engine_core::ecs::assets::load_material_definitions;
use engine_core::ecs::world::wasm::{InputSource, WasmWorld, load_schemas_from_dir};
use engine_core::worldgen::ThreadSafeWorldgenRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wasmtime::error::Context as WasmContext;
use wasmtime::{Engine, Extern, Func, Instance, Linker, Module, Store, Val};

/// A function to register host imports
pub type HostImportRegistrar = Box<dyn Fn(&mut Linker<Arc<Mutex<WasmWorld>>>) + Send + Sync>;

/// A value in the Wasm world
#[derive(Debug, Clone, PartialEq)]
pub enum WasmValue {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
}

impl From<i32> for WasmValue {
    fn from(v: i32) -> Self {
        WasmValue::I32(v)
    }
}
impl From<i64> for WasmValue {
    fn from(v: i64) -> Self {
        WasmValue::I64(v)
    }
}
impl From<f32> for WasmValue {
    fn from(v: f32) -> Self {
        WasmValue::F32(v)
    }
}
impl From<f64> for WasmValue {
    fn from(v: f64) -> Self {
        WasmValue::F64(v)
    }
}

impl From<WasmValue> for Val {
    fn from(v: WasmValue) -> Self {
        match v {
            WasmValue::I32(i) => Val::I32(i),
            WasmValue::I64(i) => Val::I64(i),
            WasmValue::F32(f) => Val::F32(f.to_bits()),
            WasmValue::F64(f) => Val::F64(f.to_bits()),
        }
    }
}

impl TryFrom<Val> for WasmValue {
    type Error = anyhow::Error;
    fn try_from(v: Val) -> Result<Self> {
        Ok(match v {
            Val::I32(i) => WasmValue::I32(i),
            Val::I64(i) => WasmValue::I64(i),
            Val::F32(f) => WasmValue::F32(f32::from_bits(f)),
            Val::F64(f) => WasmValue::F64(f64::from_bits(f)),
            _ => anyhow::bail!("Unsupported WASM value type"),
        })
    }
}

/// Known export name constants that the WASM guest may provide.
pub const EXPORT_WORLDGEN_GENERATE: &str = "mge_worldgen_generate";
pub const EXPORT_WORLDGEN_VALIDATE: &str = "mge_worldgen_validate";
pub const EXPORT_WORLDGEN_POSTPROCESS: &str = "mge_worldgen_postprocess";
pub const EXPORT_VALIDATE_MAP: &str = "mge_validate_map";
pub const EXPORT_POSTPROCESS_MAP: &str = "mge_postprocess_map";

/// Shared registration context handed to every API module.
///
/// Carries the worldgen registry for the `worldgen` domain; every other
/// domain registers from the linker alone, so the context stays minimal.
pub struct WasmApiContext {
    /// Engine worldgen registry for the `worldgen` domain.
    pub worldgen_registry: Arc<Mutex<ThreadSafeWorldgenRegistry>>,
}

/// One WASM host-API domain.
///
/// Adding a domain is one new `host_api` module file plus one unit struct
/// here plus one entry in `API_MODULES`. Transports stay thin and separate
/// per bridge; only this protocol (name + registration entry point) is
/// shared.
pub trait ApiModule: Sync {
    /// Canonical domain name, shared with the other bridges where the domain
    /// exists on both (see `engine_core::api_registry::SHARED_API_DOMAINS`).
    fn name(&self) -> &'static str;
    /// Registers the domain's host imports into the linker.
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        ctx: &WasmApiContext,
    ) -> anyhow::Result<()>;
}

// Unit structs below adapt each domain's existing `register_*` free function
// to the `ApiModule` protocol. The free functions keep their focused
// signatures; only the registry speaks the uniform shape.

/// Entity domain.
pub struct EntityModule;
impl ApiModule for EntityModule {
    fn name(&self) -> &'static str {
        "entity"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_entity_api(linker)
    }
}

/// Component domain.
pub struct ComponentModule;
impl ApiModule for ComponentModule {
    fn name(&self) -> &'static str {
        "component"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_component_api(linker)
    }
}

/// Construction domain.
pub struct ConstructionModule;
impl ApiModule for ConstructionModule {
    fn name(&self) -> &'static str {
        "construction"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_construction_api(linker)
    }
}

/// Turn domain.
pub struct TurnModule;
impl ApiModule for TurnModule {
    fn name(&self) -> &'static str {
        "turn"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_turn_api(linker)
    }
}

/// Game mode domain.
pub struct ModeModule;
impl ApiModule for ModeModule {
    fn name(&self) -> &'static str {
        "mode"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_mode_api(linker)
    }
}

/// Death/decay domain.
pub struct DeathDecayModule;
impl ApiModule for DeathDecayModule {
    fn name(&self) -> &'static str {
        "death_decay"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_death_decay_api(linker)
    }
}

/// Dungeon generation domain.
pub struct DungeonModule;
impl ApiModule for DungeonModule {
    fn name(&self) -> &'static str {
        "dungeon"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_dungeon_api(linker)
    }
}

/// Time-of-day domain.
pub struct TimeOfDayModule;
impl ApiModule for TimeOfDayModule {
    fn name(&self) -> &'static str {
        "time_of_day"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_time_of_day_api(linker)
    }
}

/// Input domain.
pub struct InputModule;
impl ApiModule for InputModule {
    fn name(&self) -> &'static str {
        "input"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_input_api(linker)
    }
}

/// Inventory domain.
pub struct InventoryModule;
impl ApiModule for InventoryModule {
    fn name(&self) -> &'static str {
        "inventory"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_inventory_api(linker)
    }
}

/// Save/load domain.
pub struct SaveLoadModule;
impl ApiModule for SaveLoadModule {
    fn name(&self) -> &'static str {
        "save_load"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_save_load_api(linker)
    }
}

/// Camera domain.
pub struct CameraModule;
impl ApiModule for CameraModule {
    fn name(&self) -> &'static str {
        "camera"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_camera_api(linker)
    }
}

/// Event bus domain.
pub struct EventBusModule;
impl ApiModule for EventBusModule {
    fn name(&self) -> &'static str {
        "event_bus"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_event_bus_api(linker)
    }
}

/// System domain.
pub struct SystemModule;
impl ApiModule for SystemModule {
    fn name(&self) -> &'static str {
        "system"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_system_api(linker)
    }
}

/// Movement operations domain.
pub struct MovementOpsModule;
impl ApiModule for MovementOpsModule {
    fn name(&self) -> &'static str {
        "movement_ops"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_movement_ops_api(linker)
    }
}

/// Equipment domain.
pub struct EquipmentModule;
impl ApiModule for EquipmentModule {
    fn name(&self) -> &'static str {
        "equipment"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_equipment_api(linker)
    }
}

/// Region domain.
pub struct RegionModule;
impl ApiModule for RegionModule {
    fn name(&self) -> &'static str {
        "region"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_region_api(linker)
    }
}

/// Body domain.
pub struct BodyModule;
impl ApiModule for BodyModule {
    fn name(&self) -> &'static str {
        "body"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_body_api(linker)
    }
}

/// Body part damage domain.
pub struct BodyPartDamageModule;
impl ApiModule for BodyPartDamageModule {
    fn name(&self) -> &'static str {
        "body_part_damage"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_body_part_damage_api(linker)
    }
}

/// Economic domain.
pub struct EconomicModule;
impl ApiModule for EconomicModule {
    fn name(&self) -> &'static str {
        "economic"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_economic_api(linker)
    }
}

/// Crafting domain.
pub struct CraftModule;
impl ApiModule for CraftModule {
    fn name(&self) -> &'static str {
        "craft"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_craft_api(linker)
    }
}

/// Job system domain.
pub struct JobSystemModule;
impl ApiModule for JobSystemModule {
    fn name(&self) -> &'static str {
        "job_system"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_system_api(linker)
    }
}

/// Job board domain.
pub struct JobBoardModule;
impl ApiModule for JobBoardModule {
    fn name(&self) -> &'static str {
        "job_board"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_board_api(linker)
    }
}

/// Job query domain.
pub struct JobQueryModule;
impl ApiModule for JobQueryModule {
    fn name(&self) -> &'static str {
        "job_query"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_query_api(linker)
    }
}

/// Job mutation domain.
pub struct JobMutationModule;
impl ApiModule for JobMutationModule {
    fn name(&self) -> &'static str {
        "job_mutation"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_mutation_api(linker)
    }
}

/// Job cancel domain.
pub struct JobCancelModule;
impl ApiModule for JobCancelModule {
    fn name(&self) -> &'static str {
        "job_cancel"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_cancel_api(linker)
    }
}

/// Job events domain.
pub struct JobEventsModule;
impl ApiModule for JobEventsModule {
    fn name(&self) -> &'static str {
        "job_events"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_events_api(linker)
    }
}

/// Job AI domain.
pub struct JobAiModule;
impl ApiModule for JobAiModule {
    fn name(&self) -> &'static str {
        "job_ai"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_job_ai_api(linker)
    }
}

/// Map domain.
pub struct MapModule;
impl ApiModule for MapModule {
    fn name(&self) -> &'static str {
        "map"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_map_api(linker)
    }
}

/// Multi-scale map navigation domain.
pub struct MultiscaleMapModule;
impl ApiModule for MultiscaleMapModule {
    fn name(&self) -> &'static str {
        "multiscale_map"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_multiscale_map_api(linker)
    }
}

/// World userdata domain.
pub struct WorldUserdataModule;
impl ApiModule for WorldUserdataModule {
    fn name(&self) -> &'static str {
        "world_userdata"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_world_userdata_api(linker)
    }
}

/// UI domain.
pub struct UiModule;
impl ApiModule for UiModule {
    fn name(&self) -> &'static str {
        "ui"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_ui_api(linker)
    }
}

/// UI tree domain.
pub struct UiTreeModule;
impl ApiModule for UiTreeModule {
    fn name(&self) -> &'static str {
        "ui_tree"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_ui_tree_api(linker)
    }
}

/// UI events domain.
pub struct UiEventsModule;
impl ApiModule for UiEventsModule {
    fn name(&self) -> &'static str {
        "ui_events"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_ui_events_api(linker)
    }
}

/// Loot domain.
pub struct LootModule;
impl ApiModule for LootModule {
    fn name(&self) -> &'static str {
        "loot"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_loot_api(linker)
    }
}

/// Material domain.
pub struct MaterialModule;
impl ApiModule for MaterialModule {
    fn name(&self) -> &'static str {
        "material"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_material_api(linker)
    }
}

/// Faction domain.
pub struct FactionModule;
impl ApiModule for FactionModule {
    fn name(&self) -> &'static str {
        "faction"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_faction_api(linker)
    }
}

/// Field-of-view domain.
pub struct FovModule;
impl ApiModule for FovModule {
    fn name(&self) -> &'static str {
        "fov"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_fov_api(linker)
    }
}

/// Noise and detection domain.
pub struct NoiseModule;
impl ApiModule for NoiseModule {
    fn name(&self) -> &'static str {
        "noise"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_noise_api(linker)
    }
}

/// Tech tree domain.
pub struct TechTreeModule;
impl ApiModule for TechTreeModule {
    fn name(&self) -> &'static str {
        "tech_tree"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_tech_tree_api(linker)
    }
}

/// Unit template domain.
pub struct UnitTemplateModule;
impl ApiModule for UnitTemplateModule {
    fn name(&self) -> &'static str {
        "unit_template"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_unit_template_api(linker)
    }
}

/// Vehicle domain.
pub struct VehicleModule;
impl ApiModule for VehicleModule {
    fn name(&self) -> &'static str {
        "vehicle"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_vehicle_api(linker)
    }
}

/// Weather domain.
pub struct WeatherModule;
impl ApiModule for WeatherModule {
    fn name(&self) -> &'static str {
        "weather"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_weather_api(linker)
    }
}

/// Temperature domain.
pub struct TemperatureModule;
impl ApiModule for TemperatureModule {
    fn name(&self) -> &'static str {
        "temperature"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_temperature_api(linker)
    }
}

/// Trade domain.
pub struct TradeModule;
impl ApiModule for TradeModule {
    fn name(&self) -> &'static str {
        "trade"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_trade_api(linker)
    }
}

/// Supply domain.
pub struct SupplyModule;
impl ApiModule for SupplyModule {
    fn name(&self) -> &'static str {
        "supply"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_supply_api(linker)
    }
}

/// Designer domain.
pub struct DesignerModule;
impl ApiModule for DesignerModule {
    fn name(&self) -> &'static str {
        "designer"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_designer_api(linker)
    }
}

/// Diplomacy domain.
pub struct DiplomacyModule;
impl ApiModule for DiplomacyModule {
    fn name(&self) -> &'static str {
        "diplomacy"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_diplomacy_api(linker)
    }
}

/// Narrative domain.
pub struct NarrativeModule;
impl ApiModule for NarrativeModule {
    fn name(&self) -> &'static str {
        "narrative"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_narrative_api(linker)
    }
}

/// Lore domain.
pub struct LoreModule;
impl ApiModule for LoreModule {
    fn name(&self) -> &'static str {
        "lore"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        _ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_lore_api(linker)
    }
}

/// Worldgen domain.
pub struct WorldgenModule;
impl ApiModule for WorldgenModule {
    fn name(&self) -> &'static str {
        "worldgen"
    }
    fn register_wasm(
        &self,
        linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
        ctx: &WasmApiContext,
    ) -> anyhow::Result<()> {
        register_worldgen_api(linker, ctx.worldgen_registry.clone())
    }
}

static ENTITY_MODULE: EntityModule = EntityModule;
static COMPONENT_MODULE: ComponentModule = ComponentModule;
static CONSTRUCTION_MODULE: ConstructionModule = ConstructionModule;
static TURN_MODULE: TurnModule = TurnModule;
static MODE_MODULE: ModeModule = ModeModule;
static DEATH_DECAY_MODULE: DeathDecayModule = DeathDecayModule;
static DUNGEON_MODULE: DungeonModule = DungeonModule;
static TIME_OF_DAY_MODULE: TimeOfDayModule = TimeOfDayModule;
static INPUT_MODULE: InputModule = InputModule;
static INVENTORY_MODULE: InventoryModule = InventoryModule;
static SAVE_LOAD_MODULE: SaveLoadModule = SaveLoadModule;
static CAMERA_MODULE: CameraModule = CameraModule;
static EVENT_BUS_MODULE: EventBusModule = EventBusModule;
static SYSTEM_MODULE: SystemModule = SystemModule;
static MOVEMENT_OPS_MODULE: MovementOpsModule = MovementOpsModule;
static EQUIPMENT_MODULE: EquipmentModule = EquipmentModule;
static REGION_MODULE: RegionModule = RegionModule;
static BODY_MODULE: BodyModule = BodyModule;
static BODY_PART_DAMAGE_MODULE: BodyPartDamageModule = BodyPartDamageModule;
static ECONOMIC_MODULE: EconomicModule = EconomicModule;
static CRAFT_MODULE: CraftModule = CraftModule;
static JOB_SYSTEM_MODULE: JobSystemModule = JobSystemModule;
static JOB_BOARD_MODULE: JobBoardModule = JobBoardModule;
static JOB_QUERY_MODULE: JobQueryModule = JobQueryModule;
static JOB_MUTATION_MODULE: JobMutationModule = JobMutationModule;
static JOB_CANCEL_MODULE: JobCancelModule = JobCancelModule;
static JOB_EVENTS_MODULE: JobEventsModule = JobEventsModule;
static JOB_AI_MODULE: JobAiModule = JobAiModule;
static MAP_MODULE: MapModule = MapModule;
static MULTISCALE_MAP_MODULE: MultiscaleMapModule = MultiscaleMapModule;
static WORLD_USERDATA_MODULE: WorldUserdataModule = WorldUserdataModule;
static UI_MODULE: UiModule = UiModule;
static UI_TREE_MODULE: UiTreeModule = UiTreeModule;
static UI_EVENTS_MODULE: UiEventsModule = UiEventsModule;
static LOOT_MODULE: LootModule = LootModule;
static MATERIAL_MODULE: MaterialModule = MaterialModule;
static FACTION_MODULE: FactionModule = FactionModule;
static FOV_MODULE: FovModule = FovModule;
static NOISE_MODULE: NoiseModule = NoiseModule;
static TECH_TREE_MODULE: TechTreeModule = TechTreeModule;
static UNIT_TEMPLATE_MODULE: UnitTemplateModule = UnitTemplateModule;
static VEHICLE_MODULE: VehicleModule = VehicleModule;
static WEATHER_MODULE: WeatherModule = WeatherModule;
static TEMPERATURE_MODULE: TemperatureModule = TemperatureModule;
static TRADE_MODULE: TradeModule = TradeModule;
static SUPPLY_MODULE: SupplyModule = SupplyModule;
static DESIGNER_MODULE: DesignerModule = DesignerModule;
static DIPLOMACY_MODULE: DiplomacyModule = DiplomacyModule;
static NARRATIVE_MODULE: NarrativeModule = NarrativeModule;
static LORE_MODULE: LoreModule = LoreModule;
static WORLDGEN_MODULE: WorldgenModule = WorldgenModule;

/// The WASM bridge API-module registry: one entry per domain, in registration
/// order. Adding a domain appends one entry here; removing an entry
/// unregisters that domain's host imports.
pub static API_MODULES: &[&dyn ApiModule] = &[
    &ENTITY_MODULE,
    &COMPONENT_MODULE,
    &CONSTRUCTION_MODULE,
    &TURN_MODULE,
    &MODE_MODULE,
    &DEATH_DECAY_MODULE,
    &DUNGEON_MODULE,
    &TIME_OF_DAY_MODULE,
    &INPUT_MODULE,
    &INVENTORY_MODULE,
    &SAVE_LOAD_MODULE,
    &CAMERA_MODULE,
    &EVENT_BUS_MODULE,
    &SYSTEM_MODULE,
    &MOVEMENT_OPS_MODULE,
    &EQUIPMENT_MODULE,
    &REGION_MODULE,
    &BODY_MODULE,
    &BODY_PART_DAMAGE_MODULE,
    &ECONOMIC_MODULE,
    &CRAFT_MODULE,
    &JOB_SYSTEM_MODULE,
    &JOB_BOARD_MODULE,
    &JOB_QUERY_MODULE,
    &JOB_MUTATION_MODULE,
    &JOB_CANCEL_MODULE,
    &JOB_EVENTS_MODULE,
    &JOB_AI_MODULE,
    &MAP_MODULE,
    &MULTISCALE_MAP_MODULE,
    &WORLD_USERDATA_MODULE,
    &UI_MODULE,
    &UI_TREE_MODULE,
    &UI_EVENTS_MODULE,
    &LOOT_MODULE,
    &MATERIAL_MODULE,
    &FACTION_MODULE,
    &FOV_MODULE,
    &NOISE_MODULE,
    &TECH_TREE_MODULE,
    &UNIT_TEMPLATE_MODULE,
    &VEHICLE_MODULE,
    &WEATHER_MODULE,
    &TEMPERATURE_MODULE,
    &TRADE_MODULE,
    &SUPPLY_MODULE,
    &DESIGNER_MODULE,
    &DIPLOMACY_MODULE,
    &NARRATIVE_MODULE,
    &LORE_MODULE,
    &WORLDGEN_MODULE,
];

/// Returns the canonical names of the given modules, in order.
pub fn api_module_names(modules: &[&dyn ApiModule]) -> Vec<&'static str> {
    modules.iter().map(|m| m.name()).collect()
}

/// Registers the given modules into the linker, in order.
pub fn register_wasm_apis(
    linker: &mut Linker<Arc<Mutex<WasmWorld>>>,
    ctx: &WasmApiContext,
    modules: &[&dyn ApiModule],
) -> anyhow::Result<()> {
    for module in modules {
        module.register_wasm(linker, ctx)?;
    }
    Ok(())
}

/// Configuration for a WASM engine
pub struct WasmScriptEngineConfig {
    /// Path to the WASM module
    pub module_path: PathBuf,
    /// Optional path to schema directory (loads *.json files into WasmWorld.component_schemas)
    pub schema_path: Option<PathBuf>,
    /// Optional worldgen registry for list/invoke host functions
    pub worldgen_registry: Option<Arc<Mutex<ThreadSafeWorldgenRegistry>>>,
    /// Optional host function registrar
    pub import_host_functions: Option<HostImportRegistrar>,
    /// Injectable input source for `get_user_input()`. Defaults to `Stdin`.
    pub input_source: Option<InputSource>,
}

/// A WASM script engine
pub struct WasmScriptEngine {
    store: Mutex<Store<Arc<Mutex<WasmWorld>>>>,
    instance: Instance,
    /// Discovered exports from the WASM module, keyed by export name.
    discovered_exports: HashMap<String, Func>,
}

impl WasmScriptEngine {
    /// Create a new WASM script engine
    pub fn new(config: WasmScriptEngineConfig) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, &config.module_path).map_err(|e| {
            e.context(format!(
                "Failed to load WASM module: {:?}",
                config.module_path
            ))
        })?;

        let mut linker = Linker::new(&engine);
        // Load schemas if schema_path is provided
        let schemas = config
            .schema_path
            .as_ref()
            .map(|p| load_schemas_from_dir(p))
            .unwrap_or_default();

        // Every host-API domain registers through the single ApiModule
        // registry; adding or removing a domain is one entry in API_MODULES.
        // The worldgen registry uses the provided handle or an empty default.
        let worldgen_registry = config
            .worldgen_registry
            .unwrap_or_else(|| Arc::new(Mutex::new(ThreadSafeWorldgenRegistry::default())));
        engine_core::worldgen::register_builtin_mapgen_algorithms(
            &mut worldgen_registry.lock().unwrap(),
        );
        let api_ctx = WasmApiContext {
            worldgen_registry: Arc::clone(&worldgen_registry),
        };
        register_wasm_apis(&mut linker, &api_ctx, API_MODULES)?;

        if let Some(imports) = config.import_host_functions {
            imports(&mut linker);
        }

        let mut world = WasmWorld::new();
        world.component_schemas = schemas;
        if let Some(src) = config.input_source {
            world.input_source = src;
        }

        // Auto-load material definitions from a "materials" sibling directory
        // relative to the schema path (e.g., engine/assets/schemas → engine/assets/materials).
        if let Some(ref schema_dir) = config.schema_path
            && let Some(parent) = schema_dir.parent()
        {
            let materials_dir = parent.join("materials");
            if let Ok(mats) = load_material_definitions(&materials_dir) {
                world.material_definitions = mats;
            }
        }

        let world = Arc::new(Mutex::new(world));
        let mut store = Store::new(&engine, world.clone());
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| e.context("Failed to instantiate WASM module"))?;

        // Scan exports for known function names and populate discovered_exports
        let discovered_exports: HashMap<String, Func> = instance
            .exports(&mut store)
            .filter_map(|export| {
                let name = export.name().to_string();
                match export.into_extern() {
                    Extern::Func(func) => Some((name, func)),
                    _ => None,
                }
            })
            .collect();

        // Store discovered export names in WasmWorld for host function access
        {
            let mut world_guard = world.lock().unwrap();
            world_guard.discovered_export_names = discovered_exports.keys().cloned().collect();
        }

        Ok(Self {
            store: Mutex::new(store),
            instance,
            discovered_exports,
        })
    }

    /// Invoke an exported function
    pub fn invoke_exported_function(
        &self,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Option<WasmValue>> {
        let mut store_guard = self.store.lock().unwrap();

        let func = self
            .instance
            .get_func(&mut *store_guard, func_name)
            .with_context(|| format!("Exported function '{func_name}' not found"))?;

        let ty = func.ty(&mut *store_guard);
        if ty.params().len() != args.len() {
            anyhow::bail!(
                "Function '{}' expects {} args, got {}",
                func_name,
                ty.params().len(),
                args.len()
            );
        }
        let vals: Vec<Val> = args.iter().cloned().map(Into::into).collect();

        let mut results = Vec::with_capacity(ty.results().len());
        for result in ty.results() {
            results.push(match result {
                wasmtime::ValType::I32 => Val::I32(0),
                wasmtime::ValType::I64 => Val::I64(0),
                wasmtime::ValType::F32 => Val::F32(0),
                wasmtime::ValType::F64 => Val::F64(0),
                _ => anyhow::bail!("Unsupported result type"),
            });
        }

        func.call(&mut *store_guard, &vals, &mut results)?;

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WasmValue::try_from(results[0])?))
        }
    }

    /// Call a discovered export by name, returning the first result value.
    /// The Func is cloned out of the map before locking the store to avoid
    /// simultaneous borrows of `self`.
    pub fn call_export(&self, name: &str, params: &[Val]) -> Result<Val, String> {
        let func = self
            .discovered_exports
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Export '{name}' not found"))?;

        let mut store_guard = self.store.lock().unwrap();
        let ty = func.ty(&mut *store_guard);

        let mut results: Vec<Val> = ty
            .results()
            .map(|rt| match rt {
                wasmtime::ValType::I32 => Val::I32(0),
                wasmtime::ValType::I64 => Val::I64(0),
                wasmtime::ValType::F32 => Val::F32(0),
                wasmtime::ValType::F64 => Val::F64(0),
                _ => Val::I32(0),
            })
            .collect();

        func.call(&mut *store_guard, params, &mut results)
            .map_err(|e| format!("Export '{}' call failed: {e}", name))?;

        results
            .into_iter()
            .next()
            .ok_or_else(|| format!("Export '{name}' returned no results"))
    }
}
