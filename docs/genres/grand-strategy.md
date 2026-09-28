# Grand Strategy

**Exemplar games**: Hearts of Iron, Europa Universalis, Victoria

Large-scale strategy with nations, warfare, diplomacy, production.

## Required engine capabilities

- Multi-scale map navigation (strategic ↔ tactical)
- Unit and equipment designer
- Supply and logistics network
- Manufacturing and production queue
- Faction and reputation system
- Diplomacy AI (relationships, treaties, war)
- Tech tree and research system
- Resource economy
- Economic engine
- Vehicle support
- Region, province, territory map system
- Fog of war and visibility system
- Skill and attribute system
- AI behaviors

## Implementation traceability — Diplomacy

The Diplomacy requirement above is implemented by genre-agnostic core and
exposed on all three scripting bridges; no grand-strategy-specific core exists.

- Store: `DiplomacyState` in `engine/core/src/diplomacy.rs` (bilateral relation
  state plus clamped standing, treaty propose/accept/break/expire lifecycle).
- System: `DiplomacySystem` in `engine/core/src/systems/diplomacy.rs`
  (per-tick treaty expiry; ordered after `FactionReputationSystem`, before
  `EnemyBehaviorSystem` in `engine/core/src/systems/mod.rs`).
- War-gate: `EnemyBehaviorSystem` in
  `engine/core/src/systems/enemy_behavior.rs` gates hostile targeting on the
  diplomatic relation (`get_relation(...) == RelationState::War`).
- Bridges: Lua (`engine_lua/src/lua_api/diplomacy.rs`), Python
  (`engine_py/src/python_api/diplomacy.rs`), WASM
  (`engine_wasm/src/host_api/diplomacy.rs`) — identical API surface
  (relation query, standing modification, treaty propose/accept, war/peace
  declaration).
- Tests: `engine/core/tests/test_diplomacy.rs`,
  `engine/scripts/lua/tests/test_diplomacy.lua`,
  `engine_py/tests/test_diplomacy.py`,
  `engine_wasm/tests/wasm_diplomacy_api.rs` with guest
  `engine_wasm/wasm_tests/test_diplomacy_api.rs`.
- Strategy-scenario coverage: `test_grand_strategy_diplomacy` in the Lua,
  Python, and WASM suites drives the register → standing → treaty → war →
  peace lifecycle through a multi-faction scenario via the public API only.
- Mode registration: `engine/assets/schemas/diplomacy.json`,
  `engine/assets/schemas/treaty.json`, and `allowed_modes` in `game.toml`
  all list `grand-strategy` (and `4x`).
