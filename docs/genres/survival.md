# Survival

**Exemplar games**: Cataclysm: Dark Days Ahead, Project Zomboid, UnReal World

Open-world survival with crafting, exploration, dynamic world systems.

## Required engine capabilities

- Z-level / multi-layer map support
- Multi-scale map navigation (overmap ↔ local)
- Skill and attribute system
- Material and property system
- Weather and climate system
- Fog of war and visibility system
- Noise and detection mechanics
- Ecosystem and wildlife simulation
- Vehicle support
- Crafting system
- Manufacturing and production queue
- Building and construction system
- Field-of-view and lighting simulation
- Procedural dungeon generation
- Item generation and loot tables
- Temperature and environment simulation
- Time-of-day and season cycle
- Inventory management
- Equipment and gear system
- Body management and equipment synchronization
- Combat and damage system
- Body part damage model
- Tech tree and research system

## Implementation traceability — Crafting system

The Crafting system requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no survival-specific core exists.

- Store: `CraftOrder` component in `engine/assets/schemas/craft_order.json` plus recipe
  definitions from `engine/core/src/systems/economic/recipe.rs` (tool-gated,
  material/quality-aware orders; stockpile-only recipes stay on the `EconomicSystem` path).
- System: `CraftingSystem` in `engine/core/src/systems/crafting.rs` (per-tick deterministic
  `CraftOrder` progress with collect-then-apply writes; ordered after `SupplySystem`, before
  `ConstructionSystem` in `engine/core/src/systems/mod.rs`).
- War-gate: N/A — craft gating is by tool, material, skill, and station, never by faction
  relation.
- Bridges: Lua (`engine_lua/src/lua_api/craft.rs`), Python
  (`engine_py/src/python_api/craft.rs`), WASM (`engine_wasm/src/host_api/craft.rs`) —
  identical craft-order start/progress/cancel/query surface.
- Tests: `engine/core/tests/test_crafting.rs` and `engine/core/tests/test_crafting_ordering.rs`,
  `engine/scripts/lua/tests/test_crafting.lua`,
  `engine_py/tests/test_crafting.py`,
  `engine_wasm/tests/wasm_craft_api.rs` with guest `engine_wasm/wasm_tests/test_craft_api.rs`.
- Survival-scenario coverage: `advances_craft_through_simulation_tick` (with
  `save_load_roundtrip_preserves_recipes_orders_entities_xp`) drives order → materials →
  single item entity via the public API only.
- Mode registration: `engine/assets/schemas/craft_order.json`
  (`"modes": ["colony", "roguelike"]`) and `allowed_modes` in `game.toml` lists
  `roguelike` (the open-world survival play mode).

## Implementation traceability — Vehicle support

The Vehicle support requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no survival-specific core exists.

- Store: `Vehicle` component in `engine/assets/schemas/vehicle.json` (occupants array as the
  single source of mount truth, plus capacity, speed, move-path, and blocked-terrains).
- System: `VehicleSystem` in `engine/core/src/systems/vehicle.rs` (per-tick embark/disembark
  state plus mounted co-movement with per-step walkability validation; ordered after
  `MovementSystem`, before `ProcessDeaths` in `engine/core/src/systems/mod.rs`).
- War-gate: province-topology vehicles hold position (documented no-step) with riders synced;
  unwalkable steps truncate the path and emit `vehicle_move_blocked`.
- Bridges: Lua (`engine_lua/src/lua_api/vehicle.rs`), Python
  (`engine_py/src/python_api/vehicle.rs`), WASM (`engine_wasm/src/host_api/vehicle.rs`) —
  identical embark/disembark/path-assign/occupancy-query surface.
- Tests: `engine/core/tests/test_vehicle.rs` and `engine/core/tests/test_vehicle_ordering.rs`,
  `engine/scripts/lua/tests/test_vehicle.lua`,
  `engine_py/tests/test_vehicle.py`,
  `engine_wasm/tests/wasm_vehicle_api.rs` with guest
  `engine_wasm/wasm_tests/test_vehicle_api.rs`.
- Survival-scenario coverage:
  `test_mounted_comovement_advances_one_cell_per_tick_with_riders_colocated` (with
  `test_save_load_roundtrip_preserves_mounted_vehicle`) drives embark → mounted travel →
  disembark via the public API only.
- Mode registration: `engine/assets/schemas/vehicle.json`
  (`"modes": ["colony", "roguelike", "simulation"]`) and `allowed_modes` in `game.toml`
  lists `roguelike` (the open-world survival play mode).

## Implementation traceability — Field-of-view and lighting simulation

The Field-of-view and lighting simulation requirement above is implemented by
genre-agnostic core and exposed on all three scripting bridges; no survival-specific core
exists.

- Store: `Sight` component in `engine/assets/schemas/sight.json` plus per-entity visible-cell
  sets in `world.visible_cells` (recomputed from `Position` each tick).
- System: `FovUpdateSystem` in `engine/core/src/systems/fov.rs` (per-tick FOV with
  topology-auto-selected algorithm — recursive shadowcasting on square, BFS flood-fill on
  hex/province; ordered after `TemperatureSystem`, before `NoiseSystem` in
  `engine/core/src/systems/mod.rs`).
- War-gate: opaque cells (`transparent: false`) block line-of-sight under the same rule the
  noise system reuses for sound blocking.
- Bridges: Lua (`engine_lua/src/lua_api/fov.rs`), Python
  (`engine_py/src/python_api/fov.rs`), WASM (`engine_wasm/src/host_api/fov.rs`, registered
  in `engine_wasm/src/engine.rs`) — identical visible-cells query, visibility check, and
  sight/algorithm setter surface.
- Tests: `engine/core/tests/test_fov.rs`,
  `engine/scripts/lua/tests/test_fov.lua`,
  `engine_py/tests/test_fov.py`,
  `engine_wasm/tests/fov.rs` (`test_wasm_fov_api_bridge`) with guest
  `engine_wasm/wasm_tests/test_fov_api.rs`.
- Survival-scenario coverage: `integration_fov_update_system_with_sight` (with
  `integration_wall_blocks_los`) drives sight → visible set → wall-blocked visibility via
  the public API only.
- Mode registration: `engine/assets/schemas/sight.json`
  (`"modes": ["colony", "roguelike", "simulation"]`) and `allowed_modes` in `game.toml`
  lists `roguelike` (the open-world survival play mode).

## Implementation traceability — Noise and detection mechanics

The Noise and detection mechanics requirement above is implemented by genre-agnostic core
and exposed on all three scripting bridges; no survival-specific core exists.

- Store: `NoiseEmitter` component in `engine/assets/schemas/noise_emitter.json` plus
  `Hearing` in `engine/assets/schemas/hearing.json` and `Stealth` in
  `engine/assets/schemas/stealth.json` (intensity/radius emission with stealth-scaled
  effective intensity; per-cell noise map applied via `World::set_noise_map`).
- System: `NoiseSystem` in `engine/core/src/systems/noise.rs` (per-tick BFS propagation with
  linear falloff and max-aggregation; declares `FovUpdateSystem` as a dependency and is
  ordered after it, before `EnemyBehaviorSystem` in `engine/core/src/systems/mod.rs`).
- War-gate: opaque cells block propagation; hearing-based AI detection escalates
  `alert_level` (idle → investigate → chase) downstream in `EnemyBehaviorSystem`.
- Bridges: Lua (`engine_lua/src/lua_api/noise.rs`), Python
  (`engine_py/src/python_api/noise.rs`), WASM (`engine_wasm/src/host_api/noise.rs`,
  registered in `engine_wasm/src/engine.rs`) — identical emit-noise, noise-at-cell, and
  hearing get/set surface.
- Tests: `engine/core/tests/test_noise.rs`,
  `engine/scripts/lua/tests/test_noise.lua`,
  `engine_py/tests/test_noise.py` (no dedicated WASM test module — host API is wired via
  `engine_wasm/src/engine.rs`).
- Survival-scenario coverage: `test_noise_emission_propagation` (with
  `test_hearing_detection_triggers_alert` and the idle → investigate → chase escalation
  case) drives emit → propagate → AI detect via the public API only.
- Mode registration: `engine/assets/schemas/noise_emitter.json`,
  `engine/assets/schemas/hearing.json`, and `engine/assets/schemas/stealth.json` (each
  listing `roguelike` among modes) and `allowed_modes` in `game.toml` lists `roguelike`
  (the open-world survival play mode).

## Implementation traceability — Item generation and loot tables

The Item generation and loot tables requirement above is implemented by genre-agnostic
core and exposed on all three scripting bridges; no survival-specific core exists.

- Store: `LootTableRegistry` in `engine/core/src/loot.rs` (named weighted-entry tables with
  deterministic rolls) plus `ItemRegistry` in `engine/core/src/ecs/item.rs` (item-definition
  catalog backing spawned loot).
- System: none — loot is demand-driven (`LootTableRegistry::roll`), not a tick system, so it
  carries no entry in `SYSTEM_EXECUTION_ORDER` in `engine/core/src/systems/mod.rs`.
- War-gate: N/A — table rolls are deterministic and faction-blind; loot never gates on the
  diplomatic relation.
- Bridges: Lua (`engine_lua/src/lua_api/loot.rs` — define/roll/has/names/remove), Python
  (`engine_py/src/python_api/world.rs` loot-table methods plus
  `engine_py/src/python_api/designer.rs` item-definition methods), WASM
  (`engine_wasm/src/host_api/loot.rs`) — identical define-table, roll, has-table,
  list-names, and remove-table surface.
- Tests: `engine/core/tests/test_item_registry.rs` and `engine/core/tests/test_loadout.rs`,
  `engine/scripts/lua/tests/test_loot.lua` and `engine/scripts/lua/tests/test_item_registry.lua`,
  `engine_py/tests/test_loot.py`,
  `engine_wasm/tests/wasm_loot_api.rs` with guest `engine_wasm/wasm_tests/test_loot_api.rs`.
- Survival-scenario coverage: `test_register_and_get_item` (with `test_load_from_dir`)
  drives define-table → roll → spawned item entities via the public API only.
- Mode registration: `engine/assets/schemas/item.json` (`"modes": ["colony", "roguelike"]`)
  and `allowed_modes` in `game.toml` lists `roguelike` (the open-world survival play mode).
