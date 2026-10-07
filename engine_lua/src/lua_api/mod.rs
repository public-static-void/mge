//! Scripting API bridge: orchestrates registration of all Lua API subsystems.
//!
//! All API functions are registered into the same table, which is then
//! exposed to the Lua script.

/// AI Behaviors API (patrol routes, AI state management)
pub mod ai_behaviors;
/// Body API
pub mod body;
/// Camera API
pub mod camera;
/// Component API
pub mod component;
/// Construction API
pub mod construction;
/// Crafting API (register_craft_recipe, list_craft_recipes, can_craft,
/// start_craft, get_craft_state, cancel_craft)
pub mod craft;
/// Death/Decay API
pub mod death_decay;
/// Diplomacy API (get_relation, get_standing, modify_standing, declare_war,
/// declare_peace, propose_treaty, accept_treaty, break_treaty, list_treaties)
pub mod diplomacy;
/// Dungeon Generation API
pub mod dungeon;
/// Economic API
pub mod economic;
/// Entity API
pub mod entity;
/// Equipment API
pub mod equipment;
/// Equipment set designer API (apply_loadout, get_loadout, validate_equipment)
pub mod equipment_set_designer;
/// Event Bus API
pub mod event_bus;
/// Faction and Reputation API
pub mod faction;
/// Field-of-view API
pub mod fov;
/// Input API
pub mod input;
/// Inventory API
pub mod inventory;
/// Item definition designer API (load, register, get, list items)
pub mod item_definition;
/// Job AI API
pub mod job_ai;
/// Job Board API
pub mod job_board;
/// Job Cancel API
pub mod job_cancel;
/// Job Events API
pub mod job_events;
/// Job Mutation API
pub mod job_mutation;
/// Job Query API
pub mod job_query;
/// Job System API
pub mod job_system;
/// Loot API
pub mod loot;
/// Lore API (generate_founding_history, list_chronicle, get_chronicle_entry,
/// render_chronicle, chronicle_len, clear_lore_history)
pub mod lore;
/// Map API
pub mod map;
/// Material API
pub mod material;
/// Game mode API
pub mod mode;
/// Movement API
pub mod movement_ops;
/// Multi-scale map navigation API (register_map, set_active_map, get_map_names,
/// get_active_map_name, link_maps, enter_map, exit_map, map_cell, unmap_cell)
pub mod multiscale_map;
/// Narrative API (register_scenario, list_scenarios, get_scenario,
/// poll_pending_decisions, get_pending_decision, resolve_decision,
/// get_narrative_history)
pub mod narrative;
/// Noise and detection API (emit_noise, get_noise_at, set_hearing, get_hearing)
pub mod noise;
/// Region API
pub mod region;
/// Save/Load API
pub mod save_load;
/// Supply API (create_supply_link, remove_supply_link, list_supply_links,
/// set_supply_link_active, get_supply_link)
pub mod supply;
/// System API
pub mod system;
/// Tech Tree and Research API
pub mod tech_tree;
/// Temperature API (get_temperature, set_temperature)
pub mod temperature;
/// Time of Day API
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
/// Weather API (get_weather, set_weather, get_weather_visibility_modifier)
pub mod weather;
/// World API
pub mod world;
/// Worldgen API
pub mod worldgen;

use crate::input::InputProvider;
use engine_core::ecs::world::World;
use engine_core::worldgen::WorldgenRegistry;

use mlua::{Lua, RegistryKey, Result as LuaResult, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Shared registration context handed to every API module.
///
/// Bundles the three dependency shapes the bridge modules need (world
/// handle, input provider, worldgen registry) plus the Lua VM handle and the
/// Lua-system table for the `system` module, so each module registers through
/// one uniform `ApiModule` interface instead of a bespoke call signature.
pub struct LuaApiContext {
    /// ECS world handle shared by the world-bound domains.
    pub world: Rc<RefCell<World>>,
    /// Input provider for the `input` domain.
    pub input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
    /// Local worldgen registry for the `worldgen` domain.
    pub worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
    /// Lua VM handle for the `system` domain.
    pub lua: Rc<Lua>,
    /// Lua-system registry table for the `system` domain.
    pub lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
}

/// One scripting API domain.
///
/// Adding a domain is one new `lua_api` module file plus one unit struct here
/// plus one entry in `API_MODULES`. Transports stay thin and separate per
/// bridge; only this protocol (name + registration entry point) is shared.
pub trait ApiModule: Sync {
    /// Canonical domain name, shared with the other bridges where the domain
    /// exists on both (see `engine_core::api_registry::SHARED_API_DOMAINS`).
    fn name(&self) -> &'static str;
    /// Registers the domain's globals into the Lua state.
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()>;
}

// Unit structs below adapt each domain's existing `register_*` free function
// to the `ApiModule` protocol. The free functions keep their focused
// signatures; only the registry speaks the uniform shape.

/// World userdata domain.
pub struct WorldModule;
impl ApiModule for WorldModule {
    fn name(&self) -> &'static str {
        "world"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        world::register_world_api(lua, globals, ctx.world.clone())
    }
}

/// Worldgen domain.
pub struct WorldgenModule;
impl ApiModule for WorldgenModule {
    fn name(&self) -> &'static str {
        "worldgen"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        worldgen::register_worldgen_api(lua, globals, ctx.worldgen_registry.clone())
    }
}

/// Event bus domain.
pub struct EventBusModule;
impl ApiModule for EventBusModule {
    fn name(&self) -> &'static str {
        "event_bus"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        event_bus::register_event_bus_api(lua, globals, ctx.world.clone())
    }
}

/// System registration domain.
pub struct SystemModule;
impl ApiModule for SystemModule {
    fn name(&self) -> &'static str {
        "system"
    }
    fn register_lua(&self, _lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        system::register_system_functions(
            Rc::clone(&ctx.lua),
            globals,
            ctx.world.clone(),
            Rc::clone(&ctx.lua_systems),
        )
    }
}

/// Entity domain.
pub struct EntityModule;
impl ApiModule for EntityModule {
    fn name(&self) -> &'static str {
        "entity"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        entity::register_entity_api(lua, globals, ctx.world.clone())
    }
}

/// Component domain.
pub struct ComponentModule;
impl ApiModule for ComponentModule {
    fn name(&self) -> &'static str {
        "component"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        component::register_component_api(lua, globals, ctx.world.clone())
    }
}

/// Input domain.
pub struct InputModule;
impl ApiModule for InputModule {
    fn name(&self) -> &'static str {
        "input"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        input::register_input_api(lua, globals, ctx.input_provider.clone())
    }
}

/// Inventory domain.
pub struct InventoryModule;
impl ApiModule for InventoryModule {
    fn name(&self) -> &'static str {
        "inventory"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        inventory::register_inventory_api(lua, globals, ctx.world.clone())
    }
}

/// Equipment domain.
pub struct EquipmentModule;
impl ApiModule for EquipmentModule {
    fn name(&self) -> &'static str {
        "equipment"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        equipment::register_equipment_api(lua, globals, ctx.world.clone())
    }
}

/// Body domain.
pub struct BodyModule;
impl ApiModule for BodyModule {
    fn name(&self) -> &'static str {
        "body"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        body::register_body_api(lua, globals, ctx.world.clone())
    }
}

/// Region domain.
pub struct RegionModule;
impl ApiModule for RegionModule {
    fn name(&self) -> &'static str {
        "region"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        region::register_region_api(lua, globals, ctx.world.clone())
    }
}

/// Camera domain.
pub struct CameraModule;
impl ApiModule for CameraModule {
    fn name(&self) -> &'static str {
        "camera"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        camera::register_camera_api(lua, globals, ctx.world.clone())
    }
}

/// UI domain.
pub struct UiModule;
impl ApiModule for UiModule {
    fn name(&self) -> &'static str {
        "ui"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, _ctx: &LuaApiContext) -> LuaResult<()> {
        ui::register_ui_api(lua, globals)
    }
}

/// Game mode domain.
pub struct ModeModule;
impl ApiModule for ModeModule {
    fn name(&self) -> &'static str {
        "mode"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        mode::register_mode_api(lua, globals, ctx.world.clone())
    }
}

/// Turn domain.
pub struct TurnModule;
impl ApiModule for TurnModule {
    fn name(&self) -> &'static str {
        "turn"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        turn::register_turn_api(lua, globals, ctx.world.clone())
    }
}

/// Save/load domain.
pub struct SaveLoadModule;
impl ApiModule for SaveLoadModule {
    fn name(&self) -> &'static str {
        "save_load"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        save_load::register_save_load_api(lua, globals, ctx.world.clone())
    }
}

/// Death/decay domain.
pub struct DeathDecayModule;
impl ApiModule for DeathDecayModule {
    fn name(&self) -> &'static str {
        "death_decay"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        death_decay::register_death_decay_api(lua, globals, ctx.world.clone())
    }
}

/// Time-of-day domain.
pub struct TimeOfDayModule;
impl ApiModule for TimeOfDayModule {
    fn name(&self) -> &'static str {
        "time_of_day"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        time_of_day::register_time_of_day_api(lua, globals, ctx.world.clone())
    }
}

/// Map domain.
pub struct MapModule;
impl ApiModule for MapModule {
    fn name(&self) -> &'static str {
        "map"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        map::register_map_api(lua, globals, ctx.world.clone())
    }
}

/// Multi-scale map navigation domain.
pub struct MultiscaleMapModule;
impl ApiModule for MultiscaleMapModule {
    fn name(&self) -> &'static str {
        "multiscale_map"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        multiscale_map::register_multiscale_map_api(lua, globals, ctx.world.clone())
    }
}

/// Narrative domain.
pub struct NarrativeModule;
impl ApiModule for NarrativeModule {
    fn name(&self) -> &'static str {
        "narrative"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        narrative::register_narrative_api(lua, globals, ctx.world.clone())
    }
}

/// Lore domain.
pub struct LoreModule;
impl ApiModule for LoreModule {
    fn name(&self) -> &'static str {
        "lore"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        lore::register_lore_api(lua, globals, ctx.world.clone())
    }
}

/// Economic domain.
pub struct EconomicModule;
impl ApiModule for EconomicModule {
    fn name(&self) -> &'static str {
        "economic"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        economic::register_economic_api(lua, globals, ctx.world.clone())
    }
}

/// Construction domain.
pub struct ConstructionModule;
impl ApiModule for ConstructionModule {
    fn name(&self) -> &'static str {
        "construction"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        construction::register_construction_api(lua, globals, ctx.world.clone())
    }
}

/// Crafting domain.
pub struct CraftModule;
impl ApiModule for CraftModule {
    fn name(&self) -> &'static str {
        "craft"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        craft::register_craft_api(lua, globals, ctx.world.clone())
    }
}

/// Movement operations domain.
pub struct MovementOpsModule;
impl ApiModule for MovementOpsModule {
    fn name(&self) -> &'static str {
        "movement_ops"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        movement_ops::register_movement_ops_api(lua, globals, ctx.world.clone())
    }
}

/// Dungeon generation domain.
pub struct DungeonModule;
impl ApiModule for DungeonModule {
    fn name(&self) -> &'static str {
        "dungeon"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, _ctx: &LuaApiContext) -> LuaResult<()> {
        dungeon::register_dungeon_api(lua, globals)
    }
}

/// Diplomacy domain.
pub struct DiplomacyModule;
impl ApiModule for DiplomacyModule {
    fn name(&self) -> &'static str {
        "diplomacy"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        diplomacy::register_diplomacy_api(lua, globals, ctx.world.clone())
    }
}

/// Trade domain.
pub struct TradeModule;
impl ApiModule for TradeModule {
    fn name(&self) -> &'static str {
        "trade"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        trade::register_trade_api(lua, globals, ctx.world.clone())
    }
}

/// Supply domain.
pub struct SupplyModule;
impl ApiModule for SupplyModule {
    fn name(&self) -> &'static str {
        "supply"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        supply::register_supply_api(lua, globals, ctx.world.clone())
    }
}

/// Job AI domain.
pub struct JobAiModule;
impl ApiModule for JobAiModule {
    fn name(&self) -> &'static str {
        "job_ai"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_ai::register_job_ai_api(lua, globals, ctx.world.clone())
    }
}

/// Loot domain.
pub struct LootModule;
impl ApiModule for LootModule {
    fn name(&self) -> &'static str {
        "loot"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        loot::register_loot_api(lua, globals, ctx.world.clone())
    }
}

/// Faction domain.
pub struct FactionModule;
impl ApiModule for FactionModule {
    fn name(&self) -> &'static str {
        "faction"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        faction::register_faction_api(lua, globals, ctx.world.clone())
    }
}

/// Material domain.
pub struct MaterialModule;
impl ApiModule for MaterialModule {
    fn name(&self) -> &'static str {
        "material"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        material::register_material_api(lua, globals, ctx.world.clone())
    }
}

/// Tech tree domain.
pub struct TechTreeModule;
impl ApiModule for TechTreeModule {
    fn name(&self) -> &'static str {
        "tech_tree"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        tech_tree::register_tech_tree_api(lua, globals, ctx.world.clone())
    }
}

/// Field-of-view domain.
pub struct FovModule;
impl ApiModule for FovModule {
    fn name(&self) -> &'static str {
        "fov"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        fov::register_fov_api(lua, globals, ctx.world.clone())
    }
}

/// Noise and detection domain.
pub struct NoiseModule;
impl ApiModule for NoiseModule {
    fn name(&self) -> &'static str {
        "noise"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        noise::register_noise_api(lua, globals, ctx.world.clone())
    }
}

/// AI behaviors domain.
pub struct AiBehaviorsModule;
impl ApiModule for AiBehaviorsModule {
    fn name(&self) -> &'static str {
        "ai_behaviors"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        ai_behaviors::register_ai_behaviors_api(lua, globals, ctx.world.clone())
    }
}

/// Unit template domain.
pub struct UnitTemplateModule;
impl ApiModule for UnitTemplateModule {
    fn name(&self) -> &'static str {
        "unit_template"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        unit_template::register_unit_template_api(lua, globals, ctx.world.clone())
    }
}

/// Vehicle domain.
pub struct VehicleModule;
impl ApiModule for VehicleModule {
    fn name(&self) -> &'static str {
        "vehicle"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        vehicle::register_vehicle_api(lua, globals, ctx.world.clone())
    }
}

/// Item definition domain.
pub struct ItemDefinitionModule;
impl ApiModule for ItemDefinitionModule {
    fn name(&self) -> &'static str {
        "item_definition"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        item_definition::register_item_definition_api(lua, globals, ctx.world.clone())
    }
}

/// Equipment set designer domain.
pub struct EquipmentSetDesignerModule;
impl ApiModule for EquipmentSetDesignerModule {
    fn name(&self) -> &'static str {
        "equipment_set_designer"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        equipment_set_designer::register_equipment_set_designer_api(lua, globals, ctx.world.clone())
    }
}

/// Weather domain.
pub struct WeatherModule;
impl ApiModule for WeatherModule {
    fn name(&self) -> &'static str {
        "weather"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        weather::register_weather_api(lua, globals, ctx.world.clone())
    }
}

/// Temperature domain.
pub struct TemperatureModule;
impl ApiModule for TemperatureModule {
    fn name(&self) -> &'static str {
        "temperature"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        temperature::register_temperature_api(lua, globals, ctx.world.clone())
    }
}

/// Job system domain.
pub struct JobSystemModule;
impl ApiModule for JobSystemModule {
    fn name(&self) -> &'static str {
        "job_system"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_system::register_job_system_api(lua, globals, ctx.world.clone())
    }
}

/// Job board domain.
pub struct JobBoardModule;
impl ApiModule for JobBoardModule {
    fn name(&self) -> &'static str {
        "job_board"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_board::register_job_board_api(lua, globals, ctx.world.clone())
    }
}

/// Job query domain.
pub struct JobQueryModule;
impl ApiModule for JobQueryModule {
    fn name(&self) -> &'static str {
        "job_query"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_query::register_job_query_api(lua, globals, ctx.world.clone())
    }
}

/// Job mutation domain.
pub struct JobMutationModule;
impl ApiModule for JobMutationModule {
    fn name(&self) -> &'static str {
        "job_mutation"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_mutation::register_job_mutation_api(lua, globals, ctx.world.clone())
    }
}

/// Job cancel domain.
pub struct JobCancelModule;
impl ApiModule for JobCancelModule {
    fn name(&self) -> &'static str {
        "job_cancel"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_cancel::register_job_cancel_api(lua, globals, ctx.world.clone())
    }
}

/// Job events domain.
pub struct JobEventsModule;
impl ApiModule for JobEventsModule {
    fn name(&self) -> &'static str {
        "job_events"
    }
    fn register_lua(&self, lua: &Lua, globals: &Table, ctx: &LuaApiContext) -> LuaResult<()> {
        job_events::register_job_event_api(lua, globals, ctx.world.clone())
    }
}

static WORLD_MODULE: WorldModule = WorldModule;
static WORLDGEN_MODULE: WorldgenModule = WorldgenModule;
static EVENT_BUS_MODULE: EventBusModule = EventBusModule;
static SYSTEM_MODULE: SystemModule = SystemModule;
static ENTITY_MODULE: EntityModule = EntityModule;
static COMPONENT_MODULE: ComponentModule = ComponentModule;
static INPUT_MODULE: InputModule = InputModule;
static INVENTORY_MODULE: InventoryModule = InventoryModule;
static EQUIPMENT_MODULE: EquipmentModule = EquipmentModule;
static BODY_MODULE: BodyModule = BodyModule;
static REGION_MODULE: RegionModule = RegionModule;
static CAMERA_MODULE: CameraModule = CameraModule;
static UI_MODULE: UiModule = UiModule;
static MODE_MODULE: ModeModule = ModeModule;
static TURN_MODULE: TurnModule = TurnModule;
static SAVE_LOAD_MODULE: SaveLoadModule = SaveLoadModule;
static DEATH_DECAY_MODULE: DeathDecayModule = DeathDecayModule;
static TIME_OF_DAY_MODULE: TimeOfDayModule = TimeOfDayModule;
static MAP_MODULE: MapModule = MapModule;
static MULTISCALE_MAP_MODULE: MultiscaleMapModule = MultiscaleMapModule;
static NARRATIVE_MODULE: NarrativeModule = NarrativeModule;
static LORE_MODULE: LoreModule = LoreModule;
static ECONOMIC_MODULE: EconomicModule = EconomicModule;
static CONSTRUCTION_MODULE: ConstructionModule = ConstructionModule;
static CRAFT_MODULE: CraftModule = CraftModule;
static MOVEMENT_OPS_MODULE: MovementOpsModule = MovementOpsModule;
static DUNGEON_MODULE: DungeonModule = DungeonModule;
static DIPLOMACY_MODULE: DiplomacyModule = DiplomacyModule;
static TRADE_MODULE: TradeModule = TradeModule;
static SUPPLY_MODULE: SupplyModule = SupplyModule;
static JOB_AI_MODULE: JobAiModule = JobAiModule;
static LOOT_MODULE: LootModule = LootModule;
static FACTION_MODULE: FactionModule = FactionModule;
static MATERIAL_MODULE: MaterialModule = MaterialModule;
static TECH_TREE_MODULE: TechTreeModule = TechTreeModule;
static FOV_MODULE: FovModule = FovModule;
static NOISE_MODULE: NoiseModule = NoiseModule;
static AI_BEHAVIORS_MODULE: AiBehaviorsModule = AiBehaviorsModule;
static UNIT_TEMPLATE_MODULE: UnitTemplateModule = UnitTemplateModule;
static VEHICLE_MODULE: VehicleModule = VehicleModule;
static ITEM_DEFINITION_MODULE: ItemDefinitionModule = ItemDefinitionModule;
static EQUIPMENT_SET_DESIGNER_MODULE: EquipmentSetDesignerModule = EquipmentSetDesignerModule;
static WEATHER_MODULE: WeatherModule = WeatherModule;
static TEMPERATURE_MODULE: TemperatureModule = TemperatureModule;
static JOB_SYSTEM_MODULE: JobSystemModule = JobSystemModule;
static JOB_BOARD_MODULE: JobBoardModule = JobBoardModule;
static JOB_QUERY_MODULE: JobQueryModule = JobQueryModule;
static JOB_MUTATION_MODULE: JobMutationModule = JobMutationModule;
static JOB_CANCEL_MODULE: JobCancelModule = JobCancelModule;
static JOB_EVENTS_MODULE: JobEventsModule = JobEventsModule;

/// The Lua bridge API-module registry: one entry per domain, in registration
/// order. Adding a domain appends one entry here; removing an entry
/// unregisters that domain.
pub static API_MODULES: &[&dyn ApiModule] = &[
    &WORLD_MODULE,
    &WORLDGEN_MODULE,
    &EVENT_BUS_MODULE,
    &SYSTEM_MODULE,
    &ENTITY_MODULE,
    &COMPONENT_MODULE,
    &INPUT_MODULE,
    &INVENTORY_MODULE,
    &EQUIPMENT_MODULE,
    &BODY_MODULE,
    &REGION_MODULE,
    &CAMERA_MODULE,
    &UI_MODULE,
    &MODE_MODULE,
    &TURN_MODULE,
    &SAVE_LOAD_MODULE,
    &DEATH_DECAY_MODULE,
    &TIME_OF_DAY_MODULE,
    &MAP_MODULE,
    &MULTISCALE_MAP_MODULE,
    &NARRATIVE_MODULE,
    &LORE_MODULE,
    &ECONOMIC_MODULE,
    &CONSTRUCTION_MODULE,
    &CRAFT_MODULE,
    &MOVEMENT_OPS_MODULE,
    &DUNGEON_MODULE,
    &DIPLOMACY_MODULE,
    &TRADE_MODULE,
    &SUPPLY_MODULE,
    &JOB_AI_MODULE,
    &LOOT_MODULE,
    &FACTION_MODULE,
    &MATERIAL_MODULE,
    &TECH_TREE_MODULE,
    &FOV_MODULE,
    &NOISE_MODULE,
    &AI_BEHAVIORS_MODULE,
    &UNIT_TEMPLATE_MODULE,
    &VEHICLE_MODULE,
    &ITEM_DEFINITION_MODULE,
    &EQUIPMENT_SET_DESIGNER_MODULE,
    &WEATHER_MODULE,
    &TEMPERATURE_MODULE,
    &JOB_SYSTEM_MODULE,
    &JOB_BOARD_MODULE,
    &JOB_QUERY_MODULE,
    &JOB_MUTATION_MODULE,
    &JOB_CANCEL_MODULE,
    &JOB_EVENTS_MODULE,
];

/// Returns the canonical names of the given modules, in order.
pub fn api_module_names(modules: &[&dyn ApiModule]) -> Vec<&'static str> {
    modules.iter().map(|m| m.name()).collect()
}

/// Registers the given modules into the Lua globals table, in order.
pub fn register_api_modules(
    lua: &Lua,
    globals: &Table,
    ctx: &LuaApiContext,
    modules: &[&dyn ApiModule],
) -> LuaResult<()> {
    for module in modules {
        module.register_lua(lua, globals, ctx)?;
    }
    Ok(())
}

/// Registers all Lua API functions into the given globals table.
pub fn register_all_api_functions(
    lua: &Rc<Lua>,
    globals: &Table,
    world: Rc<RefCell<World>>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
    lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
) -> LuaResult<()> {
    let ctx = LuaApiContext {
        world,
        input_provider,
        worldgen_registry,
        lua: Rc::clone(lua),
        lua_systems,
    };
    register_api_modules(lua, globals, &ctx, API_MODULES)
}
