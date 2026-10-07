# Colony Sim

**Exemplar games**: Dwarf Fortress, RimWorld, Gnomoria

Deep simulation of individual creatures, complex material interactions, colony management.

## Required engine capabilities

- Z-level / multi-layer map support
- Skill and attribute system
- Material and property system
- Fluid simulation (water, magma)
- Ecosystem and wildlife simulation
- Administration and zone management
- Procedural history and lore generation
- Building and construction system
- Temperature and environment simulation
- Weather and climate system
- Job system
- Economic engine
- Event-driven narrative engine
- Inventory management
- Equipment and gear system
- Body management and equipment synchronization
- Combat and damage system
- Body part damage model
- Time-of-day and season cycle
- Crafting system
- Resource economy
- Faction and reputation system

## Implementation traceability — Job system

The Job system requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no colony-specific core exists.

- Store: `JobBoard` in `engine/core/src/systems/job/board/job_board.rs` (policy-driven
  assignment queue with claim, priority-aging, and persistence lifecycle).
- System: `JobSystem` in `engine/core/src/systems/job/` (per-tick claim → progress →
  complete assignment; ordered after `ResearchSystem`, before `EconomicSystem` in
  `engine/core/src/systems/mod.rs`).
- War-gate: N/A — assignment is faction-blind; hostile targeting stays gated downstream in
  `EnemyBehaviorSystem` on the diplomatic relation.
- Bridges: Lua (`engine_lua/src/lua_api/job_board.rs`, `job_system.rs`, `job_query.rs`,
  `job_mutation.rs`, `job_events.rs`, `job_ai.rs`, `job_cancel.rs`), Python
  (`engine_py/src/python_api/job_board.rs`, `job_api.rs`, `job_query.rs`, `job_mutation.rs`,
  `job_events.rs`, `job_ai.rs`), WASM (`engine_wasm/src/host_api/job_board.rs`,
  `job_system.rs`, `job_query.rs`, `job_mutation.rs`, `job_events.rs`, `job_ai.rs`,
  `job_cancel.rs`) — identical board, query, mutation, events, AI-assignment, and
  cancellation surface.
- Tests: `engine/core/tests/job_board_tests.rs`, `engine/core/tests/job_lifecycle_tests.rs`,
  `engine/core/tests/job_resource_tests.rs`,
  `engine/scripts/lua/tests/test_job.lua` (plus `test_job_board.lua`, `test_job_query.lua`,
  `test_job_mutation.lua`, `test_job_cancellation.lua`),
  `engine_py/tests/test_job.py` (plus `test_job_board.py`, `test_job_query.py`,
  `test_job_mutation.py`, `test_job_cancellation.py`),
  `engine_wasm/tests/wasm_job_system.rs` and `engine_wasm/tests/wasm_job_board.rs` with guests
  `engine_wasm/wasm_tests/test_job_system.rs` and `engine_wasm/wasm_tests/test_job_board.rs`.
- Colony-scenario coverage: `test_job_assignment_claims_job` (with the fairness,
  priority-aging, and persistence cases) drives post → claim → complete through a
  colony work queue via the public API only.
- Mode registration: `engine/assets/schemas/job.json` (`"modes": ["colony", "roguelike"]`)
  and `allowed_modes` in `game.toml` lists `colony`.

## Implementation traceability — Economic engine

The Economic engine requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no colony-specific core exists.

- Store: `Recipe` / `ResourceAmount` in `engine/core/src/systems/economic/recipe.rs` plus
  resource unit-weight/volume lookup in `engine/core/src/systems/economic/resource.rs`
  (validated recipe and stockpile-count records, so save/load round-trips for free).
- System: `EconomicSystem` in `engine/core/src/systems/economic/system.rs` (per-tick
  stockpile production and consumption against recipes; ordered after `JobSystem`, before
  `ConsumptionSystem` in `engine/core/src/systems/mod.rs`).
- War-gate: N/A — production and consumption are faction-blind; wartime transfer blocking
  lives downstream in `SupplySystem` (`supply_blocked {reason: war}`).
- Bridges: Lua (`engine_lua/src/lua_api/economic.rs`, `trade.rs`), Python
  (`engine_py/src/python_api/economic.rs`, `trade.rs`), WASM
  (`engine_wasm/src/host_api/economic.rs`, `trade.rs`) — identical recipe, resource,
  stockpile, and trade surface.
- Tests: `engine/core/tests/test_trade_consumption.rs`,
  `engine/scripts/lua/tests/test_economic.lua` and `engine/scripts/lua/tests/test_trade.lua`,
  `engine_py/tests/test_economic.py` and `engine_py/tests/test_trade.py`,
  `engine_wasm/tests/wasm_economic_api.rs` and `engine_wasm/tests/wasm_economic_reservation.rs`
  with guests `engine_wasm/wasm_tests/test_economic_api.rs` and
  `engine_wasm/wasm_tests/test_economic_reservation.rs`.
- Colony-scenario coverage: `moves_grain_between_stockpiles_in_one_call` (with the
  insufficient-funds no-mutation case) drives debit → credit across colony granaries via
  the public API only.
- Mode registration: `engine/assets/schemas/stockpile.json`,
  `engine/assets/schemas/resource.json`, and `engine/assets/schemas/production_job.json`
  (each `"modes": ["colony", "4x", "grand-strategy"]`) and `allowed_modes` in `game.toml`
  lists `colony`.

## Implementation traceability — Crafting system

The Crafting system requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no colony-specific core exists.

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
- Colony-scenario coverage: `advances_craft_through_simulation_tick` (with
  `save_load_roundtrip_preserves_recipes_orders_entities_xp`) drives order → materials →
  single item entity via the public API only.
- Mode registration: `engine/assets/schemas/craft_order.json`
  (`"modes": ["colony", "roguelike"]`) and `allowed_modes` in `game.toml` lists `colony`.

## Implementation traceability — Temperature and environment simulation

The Temperature and environment simulation requirement above is implemented by
genre-agnostic core and exposed on all three scripting bridges; no colony-specific core
exists.

- Store: `TemperatureState` in `engine/core/src/systems/temperature.rs` (global ambient in
  °C with script manual-override, plus humidity/pressure bands and a transient per-cell
  diffusion map).
- System: `TemperatureSystem` in `engine/core/src/systems/temperature.rs` (per-tick ambient
  derivation from season/weather/diurnal cycle plus per-body-part heat drift; ordered after
  `WeatherSystem`, before `FovUpdateSystem` in `engine/core/src/systems/mod.rs`).
- War-gate: N/A — ambient physics applies to every body regardless of faction relation.
- Bridges: Lua (`engine_lua/src/lua_api/temperature.rs`), Python
  (`engine_py/src/python_api/temperature.rs`), WASM
  (`engine_wasm/src/host_api/temperature.rs`) — identical ambient get/override, humidity,
  pressure, and cell-temperature surface.
- Tests: `engine/core/tests/test_temperature.rs`,
  `engine/scripts/lua/tests/test_temperature.lua`,
  `engine_py/tests/test_temperature.py`,
  `engine_wasm/tests/wasm_temperature_api.rs` with guest
  `engine_wasm/wasm_tests/test_temperature_api.rs`.
- Colony-scenario coverage: `ambient_formula_matches_reference_values` (with
  `body_parts_drift_toward_ambient_at_insulation_scaled_rate`) drives season → ambient →
  clothed-body drift via the public API only.
- Mode registration: `engine/assets/schemas/heat_source.json`
  (`"modes": ["colony", "roguelike", "simulation"]`) and `allowed_modes` in `game.toml`
  lists `colony`.
