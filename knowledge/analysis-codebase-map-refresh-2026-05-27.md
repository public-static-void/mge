---
title: "ANALYSIS: Codebase Map Refresh — MGE"
version: 1.0.0
status: draft
type: analysis
created: 2026-05-27
author: Explorer
superseded_by: null
---

# ANALYSIS: Codebase Map Refresh — MGE

## Landscape

Complete re-exploration of the MGE (Modular Game Engine) monorepo — a cross-language game engine with schema-driven ECS, native plugin ABI, Lua/Python/WASM scripting, terminal renderer, and job system. This KD supersedes the prior `analysis-codebase-map-2026-05-27.md`.

### Quick Facts

| Metric | Value |
|--------|-------|
| Workspace members | 9 (Cargo.toml `[workspace].members`) |
| Effective crates | 10 (9 members + 1 stale nested Cargo.lock in engine/core/) |
| Languages | Rust (edition 2024 nightly), Lua (LuaJIT), Python, C, WASM |
| Rust edition exception | plugins/rust_test_plugin uses edition 2021 |
| Build system | Cargo + Makefile + xtask orchestrator |
| CI | 3 GitHub Actions workflows |
| Release automation | semantic-release via Node.js |
| Engine core Rust source files | ~95 `.rs` files |
| Lua binding files | 39 `.rs` files |
| Python binding files | 33 `.rs` files |

## 1. Workspace Architecture

### Crate Manifest (`/Cargo.toml`)

```toml
[workspace]
members = [
  "engine/core",                    # engine_core — pure Rust ECS core
  "engine_macros",                  # engine_macros — proc-macro crate
  "engine_py",                      # engine_py — Python native extension (cdylib)
  "engine_lua",                     # engine_lua — Lua scripting bridge (2 binaries)
  "engine_wasm",                    # engine_wasm — WASM sandbox
  "engine/tools/schema_validator",  # schema_validator — CLI tool
  "plugins/rust_test_plugin",       # Rust test plugin (cdylib + bin)
  "xtask",                          # Build orchestrator
  "engine/tools/codegen",           # Code generation tool (unwired)
]
resolver = "3"
```

### Dependency Graph

```
engine_macros ← engine_core ← engine_lua
                              ← engine_py
                              ← engine_wasm
                              (depends on schema_validator as a lib)

schema_validator (standalone bin) — CLI tool
codegen (standalone bin) — unwired tool
xtask (standalone bin) — build orchestrator
rust_test_plugin (cdylib+bin) — test plugin
```

### Crate Details

| Crate | Path | Type | Edition | Key Purpose |
|-------|------|------|---------|-------------|
| engine_core | engine/core/ | lib+bin | 2024 | ECS core, map, plugins, systems, presentation, worldgen. Also a binary (plugin subprocess handler) |
| engine_macros | engine_macros/ | proc-macro | 2024 | `#[component]` attribute macro — generates versioning, migration, serde, schema |
| engine_lua | engine_lua/ | lib+2bins | 2024 | LuaJIT bridge via mlua. Binaries: mge_cli, mge_lua_test_runner |
| engine_py | engine_py/ | cdylib | 2024 | Python native extension via pyo3 + maturin |
| engine_wasm | engine_wasm/ | lib | 2024 | WASM sandbox via wasmtime (async) |
| schema_validator | engine/tools/schema_validator/ | bin | 2024 | JSON schema validation CLI |
| codegen | engine/tools/codegen/ | bin | 2024 | Code generation (unwired from build) |
| xtask | xtask/ | bin | 2024 | Plugin build/deploy orchestrator |
| rust_test_plugin | plugins/rust_test_plugin/ | cdylib+bin | 2021 | Test plugin; only edition 2021 crate |

### Feature Flags on engine_core (all empty/vestigial)

```toml
[features]
lua = []
python = []
wasm = []
```

No `cfg(feature = ...)` guards found in engine_core/src/. The flags control nothing.

### Key Dependencies

| Crate | Dependencies |
|-------|-------------|
| engine_core | engine_macros, schema_validator, mlua (LuaJIT), serde, schemars, serde_json, bson, semver, thiserror, anyhow, libloading, jsonschema, once_cell, indexmap, topo_sort, serde_yaml, toml, parking_lot, walkdir, lazy_static, dyn-clone |
| engine_macros | syn, quote, proc-macro2, bson |
| engine_lua | engine_core, mlua, serde_json, gag, regex, once_cell |
| engine_py | engine_core, pyo3, serde_json, serde-pyobject, pythonize |
| engine_wasm | engine_core, wasmtime (async), anyhow, serde_json |
| schema_validator | serde_json, clap, toml |
| codegen | serde, serde_json, schemars |
| xtask | (none — std only) |
| rust_test_plugin | serde, serde_json, libc, ctor |

## 2. Entrypoints (Binary Targets)

| Binary | Crate | Path | Purpose |
|--------|-------|------|---------|
| `mge_cli` | engine_lua | `src/bin/mge_cli.rs` (213 lines) | Main game CLI. Runs Lua scripts (direct path) or mods (`--mod` flag). Loads schemas, plugins, worldgen, mod manifests. |
| `mge_lua_test_runner` | engine_lua | `src/bin/mge_lua_test_runner.rs` | Lua test harness. Source-parses test discovery. |
| `schema_validator` | schema_validator | `tools/schema_validator/src/main.rs` (117 lines) | Validates JSON schemas against allowed modes. Args: PATH, --config, --fail-fast, --summary-only |
| `xtask` | xtask | `src/main.rs` (250 lines) | Build orchestrator. Commands: build-plugins, build-c-plugins, build-wasm-tests, build-all |
| `engine_core` (subprocess) | engine_core | `src/main.rs` (92 lines) | Plugin subprocess handler. Unix socket protocol with JSON messages. |
| `rust_test_plugin` (binary) | rust_test_plugin | `src/main.rs` | Standalone binary from test plugin crate |

### Examples

| Example | Path | Purpose |
|---------|------|---------|
| viewport_demo | engine/core/examples/viewport_demo.rs | Terminal viewport rendering |
| viewport_camera_demo | engine/core/examples/viewport_camera_demo.rs | Camera + viewport interaction |

## 3. Source Tree — engine_core Module Structure

```
engine/core/src/
├── lib.rs              # Public API: 10 top-level modules, re-exports World, Mode, components
├── main.rs             # Plugin subprocess handler (Unix socket, JSON protocol)
├── config.rs           # GameConfig (loads game.toml)
├── ecs/
│   ├── mod.rs          # Re-exports: World, ComponentRegistry, ComponentSchema
│   ├── assets.rs       # Asset loading
│   ├── error.rs        # MigrationError, RegistryError
│   ├── event.rs        # Event bus types
│   ├── event_bus_registry.rs
│   ├── event_logger.rs # Event logging system
│   ├── registry.rs     # ComponentRegistry
│   ├── schema.rs       # ComponentSchema, schema loading with mode filtering
│   ├── system.rs       # ECS system trait
│   ├── components/
│   │   ├── mod.rs
│   │   ├── camera.rs
│   │   ├── corpse.rs
│   │   ├── decay.rs
│   │   ├── happiness.rs
│   │   ├── health.rs
│   │   ├── inventory.rs
│   │   └── position.rs
│   └── world/
│       ├── mod.rs      # World struct — central ECS container
│       ├── component.rs # Component mutation on entities
│       ├── entity.rs   # Entity lifecycle (spawn/despawn)
│       ├── events.rs   # World-level event handling
│       ├── job_handlers.rs # Job handler registration
│       ├── map.rs      # World map integration
│       ├── mode.rs     # Mode enforcement
│       ├── resources.rs
│       ├── save_load.rs
│       ├── systems.rs  # System scheduling
│       └── wasm.rs     # WASM host functions
├── map/
│   ├── mod.rs
│   ├── cell_key.rs     # CellKey enum: Square, Hex, Province
│   ├── deserialize.rs  # Map deserialization
│   ├── hex.rs          # Hex map implementation
│   ├── pathfinding.rs  # A* pathfinding
│   ├── province.rs     # Province map
│   ├── square.rs       # Square map
│   └── topology.rs     # Topology trait
├── modes/
│   └── mod.rs          # GameMode enum, ModeRestrictedComponent trait
├── mods/
│   ├── mod.rs
│   ├── loader.rs       # Mod loading (schemas + system scripts)
│   └── manifest.rs     # Mod manifest parsing
├── plugins/
│   ├── mod.rs
│   ├── dynamic_systems.rs # Dynamic system registration from plugins
│   ├── ffi.rs          # FFI functions (spawn_entity, set_component)
│   ├── loader.rs       # Native plugin loading (.so files)
│   ├── manager.rs      # Plugin lifecycle management
│   ├── registry.rs     # Plugin registry
│   ├── subprocess.rs   # Subprocess plugin protocol
│   └── types.rs        # EngineApi, PluginVTable Rust bindings
├── presentation/
│   ├── mod.rs          # PresentationSystem<R> — world rendering + map rendering
│   ├── input.rs        # InputEvent enum, PresentationInput trait
│   ├── layout.rs       # CellLayout trait, SquareLayout, HexLayout, ProvinceLayout
│   ├── renderer.rs     # PresentationRenderer trait, RenderCommand, RenderColor
│   └── ui/
│       ├── mod.rs      # Re-exports all widgets, register_all_widgets()
│       ├── event.rs    # UiEvent enum (Click, KeyPress)
│       ├── factory.rs  # UiFactory, WidgetConstructor, global UI_FACTORY singleton
│       ├── root.rs     # UiRoot — root node, focus management, keyboard navigation
│       ├── schema_loader.rs # JSON→UI tree deserializer
│       ├── layout/
│       │   ├── mod.rs
│       │   ├── direction.rs
│       │   ├── grid.rs
│       │   └── linear.rs
│       └── widget/
│           ├── mod.rs          # UiNode type alias, re-exports
│           ├── widget_trait.rs # UiWidget trait, WidgetId, update_struct_from_props
│           ├── button.rs
│           ├── checkbox.rs
│           ├── context_menu.rs
│           ├── dropdown.rs
│           ├── dynamic.rs
│           ├── event_log.rs
│           ├── focus_grid.rs
│           ├── label.rs
│           ├── panel.rs
│           └── text_input.rs
├── systems/
│   ├── mod.rs          # System registration
│   ├── body_equipment_sync.rs
│   ├── death_decay.rs
│   ├── equipment_effect_aggregation.rs
│   ├── equipment_logic.rs
│   ├── inventory.rs
│   ├── movement_system.rs
│   ├── stat_calculation.rs
│   ├── economic/
│   │   ├── mod.rs, loader.rs, recipe.rs, resource.rs, system.rs
│   └── job/
│       ├── mod.rs      # Re-exports all job sub-modules
│       ├── ai/         # 4 files: mod.rs, event_intent.rs, event_reaction_system.rs, logic.rs
│       ├── board/      # 3 files: mod.rs, job_board.rs, priority_aging.rs
│       ├── core/       # 4 files: mod.rs, children.rs, dependencies.rs, requirements.rs
│       ├── ops/        # 3 files: mod.rs, movement_ops.rs, resource_ops.rs
│       ├── registry/   # 3 files: mod.rs, effect_processor_registry.rs, job_handler_registry.rs
│       ├── reservation/ # 2 files: mod.rs, resource_reservation.rs
│       ├── states/     # 5 files: mod.rs, helpers.rs, transitions.rs, movement/ (3 files), resource/ (3 files)
│       ├── system/     # 5 files: mod.rs, orchestrator.rs, process.rs, effects.rs, events.rs
│       └── types/      # 4 files: mod.rs, builtin_handlers.rs, job_type.rs, loader.rs
└── worldgen.rs         # WorldgenRegistry trait
```

### Total engine_core source files: ~95 `.rs` files

## 4. Plugin System

### Plugin Inventory

| Plugin | Language | Source | Output | Plugin Type |
|--------|----------|--------|--------|-------------|
| rust_test_plugin | Rust cdylib (edition 2021) | src/lib.rs + src/main.rs | librust_test_plugin.so | Native (C ABI via vtable) |
| simple_square_plugin | C | simple_square_plugin.c | libsimple_square_plugin.so | Native (C ABI) |
| simple_hex_plugin | C | simple_hex_plugin.c | libsimple_hex_plugin.so | Native (C ABI) |
| simple_province_plugin | C | simple_province_plugin.c | libsimple_province_plugin.so | Native (C ABI) |
| test_plugin | C | test_plugin.c | libtest_plugin.so | Native (C ABI) |

### Plugin Manifest Format (plugin.json)

All 5 plugins share an identical schema:
```json
{
  "name": "...",
  "version": "1.0.0",
  "description": "...",
  "authors": ["Test Author"],
  "dependencies": [],
  "dynamic_library": "lib<name>.so"
}
```

### Native Plugin ABI (engine/engine_plugin_abi.h)

```c
typedef struct EngineApi {
    unsigned int (*spawn_entity)(WorldPtr);
    int (*set_component)(WorldPtr, unsigned int, const char *name, const char *json_value);
} EngineApi;

typedef struct PluginVTable {
    int (*init)(EngineApi *api, void *world);
    void (*shutdown)();
    void (*update)(float delta_time);
    const char *(*worldgen_name)();
    int (*generate_world)(const char *params_json, char **out_result_json);
    void (*free_result_json)(char *result_json);
    int (*register_systems)(EngineApi *api, void *world, SystemPlugin **systems, int *count);
    void (*free_systems)(SystemPlugin *systems, int count);
    void *(*hot_reload)(void *old_state);
} PluginVTable;

extern PluginVTable *PLUGIN_VTABLE;  // Exported symbol
```

### C Plugin Pattern

Each C plugin:
1. Has a single `.c` file
2. Uses `__attribute__((constructor))` to populate vtable
3. Exports `PLUGIN_VTABLE` with `__attribute__((visibility("default")))`
4. Compiled via xtask with: `gcc -shared -fPIC -ljansson`
5. Requires libjansson-dev for JSON parsing

### Rust Plugin Pattern (rust_test_plugin)
- Uses `#[ctor::ctor]` for vtable init (Rust equivalent of `__attribute__((constructor))`)
- Must use `#[no_mangle] pub static mut PLUGIN_VTABLE`
- Compiled via `cargo build --release` in the plugin dir, then `.so` copied back

### Two Plugin Systems

1. **Native ABI**: `.so` files loaded via `libloading` (dynamic linking). PluginVTable provides init/shutdown/update/worldgen/system registration/hot-reload.
2. **Subprocess**: engine_core binary runs as a subprocess with Unix socket JSON protocol. Far less developed — primarily echoes commands.

### Plugin Bootstrap in game.toml

```toml
[plugins]
native = ["plugins/simple_square_plugin/libsimple_square_plugin.so"]
```

Only simple_square_plugin is registered by default.

## 5. Mod System

### Mod Structure (`mods/mvp_roguelike/`)

```
mods/mvp_roguelike/
├── mod.json           # Manifest
├── assets/map1.json   # Map data
├── schemas/           # 6 component schemas
│   ├── player.json
│   ├── monster.json
│   ├── item.json
│   ├── renderable.json
│   ├── position.json
│   └── health.json
├── systems/
│   └── main.lua       # Game logic (single system file)
└── README.md
```

### Mod Manifest Format (mod.json)

```json
{
  "name": "mvp_roguelike",
  "version": "1.0.0",
  "mode": "roguelike",
  "schemas": ["schemas/player.json", ...],
  "systems": [{"file": "systems/main.lua", "name": "RoguelikeMain"}],
  "main_script": "systems/main.lua"
}
```

### Mod Loading Flow (engine_core::mods::loader)

1. Load engine schemas from `engine/assets/schemas/` (filtered by allowed modes)
2. Register them into ComponentRegistry
3. Load mod manifest → merge mod-specific schemas
4. Set game mode from manifest (or CLI override)
5. Load and register plugins
6. Initialize ScriptEngine (Lua)
7. Run mod's main_script

## 6. Test Landscape

### Test File Counts

| Location | Count | Runner | Notes |
|----------|-------|--------|-------|
| engine/core/tests/ | 108 | `cargo test` | Integration tests, require C plugins + WASM |
| engine_py/tests/ | 44 | `pytest` | Requires maturin develop first |
| engine/scripts/lua/tests/ | 47 + 2 helpers | `mge_lua_test_runner` | Source-parsing test discovery |
| engine_lua/tests/ | 2 | `cargo test` | Script loading + input tests |
| engine_wasm/tests/ | 3 | `cargo test` | WASM engine integration |
| engine_macros/tests/ | 1 | `cargo test` | Proc-macro component tests |
| schema_validator/tests/ | 0-? | `cargo test` | Schema validator tests |

### Test Infrastructure Details

**Rust tests** (engine/core/tests/): 108 files covering: ai, body_equipment, component migration/registration, death_removal, economic, equipment, event_bus (10+ variants), inventory, jobs (25+ files), map (8 files), mod_loader, mode, movement, pathfinding, plugins (6 files), presentation, save_load, schema, simulation, systems, time_of_day, ui, worldgen (5 files).

**Lua tests**: 47 `.lua` files. Discovery is source-parsing based — the Rust test runner reads each `.lua` file, strips comments, parses `return { test_xxx = function() ... end }` patterns. Each test gets a fresh World instance. Pre-registered systems: ProcessDeaths, ProcessDecay, EconomicSystem, JobSystem, InventoryConstraintSystem, EquipmentLogicSystem, BodyEquipmentSyncSystem. Helpers: `helpers/job_helpers.lua`, `helpers/ai_job_helpers.lua`.

**Python tests**: 44 `.py` files using pytest. Fixture via conftest.py: `make_world()` creates PyWorld with schema dir `../../engine/assets/schemas`. Requires maturin develop.

### Test Prerequisites
- Rust tests: Pre-built C plugins + WASM test modules
- Lua tests: `libsimple_square_plugin.so` at plugins/simple_square_plugin/
- Python tests: maturin develop (builds .so) + C plugins
- All: `LD_LIBRARY_PATH` must include `$PWD/plugins`

## 7. CI Pipeline

### GitHub Actions Workflows (3 files)

#### 1. ci.yml — Main CI (push/PR to main)

```
┌─────────┐
│  lint   │  (rustfmt + clippy)
└────┬────┘
     │ (parallel)
     ├─────────────────┬──────────────────┐
     ▼                 ▼                  ▼
┌────────────┐  ┌──────────────┐  ┌──────────────┐
│validate    │  │build-c-      │  │build-wasm-   │
│-schema     │  │plugins       │  │tests         │
└────────────┘  └──────┬───────┘  └──────┬───────┘
                       │ (download)      │ (download)
                       ▼                 ▼
                    ┌─────────────────────────┐
                    │      build-all           │
                    │ (xtask build-all)        │
                    └──────────┬──────────────┘
                               ▼
                    ┌──────────────────┐
                    │    test-rust      │
                    │ (needs build-all) │
                    └──────────────────┘
                    ┌──────────────────┐
                    │    test-lua       │
                    │ (needs c-plugins) │
                    └──────────────────┘
                    ┌──────────────────┐
                    │   test-python     │
                    │ (needs c-plugins) │
                    └──────────────────┘
```

Enforced order: lint → validate-schema + build-c-plugins + build-wasm-tests → build-all → test-rust → test-lua → test-python

#### 2. lint-schemas.yml — Path-filtered schema linting
- Triggers on changes to `engine/assets/schemas/**`, `engine/tools/schema_validator/**`, or the workflow file
- Runs `schema_validator --summary-only`

#### 3. release.yml — Semantic Release on push to main
- Installs Node.js 24, runs `npx semantic-release`
- Uses @timada/semantic-release-cargo for Cargo crate publishing
- Permissions: contents:write, issues:write, pull-requests:write
- Commit format: `<type>(<scope>): <subject>` (enforced by .gitmessage)

## 8. Build System

### Makefile Targets

| Target | Action | Dependencies |
|--------|--------|-------------|
| `all` | validate-schema + build-all | — |
| `test` / `test-all` | validate-schema → test-rust → test-python → test-lua | — |
| `validate-schema` | `schema_validator -- engine/assets/schemas` | — |
| `build-plugins` | `xtask build-plugins` | — |
| `build-c-plugins` | `xtask build-c-plugins` | — |
| `build-wasm-tests` | `xtask build-wasm-tests` | — |
| `build-all` | Chains all 3 xtask builds | — |
| `test-rust` | `cargo test --all` | — |
| `test-python` | setup-python → maturin develop → pytest | setup-python |
| `test-lua` | build mge_lua_test_runner → run_lua_tests.sh | — |
| `clean` | cargo clean | — |
| `setup-python` | Creates venv, pip installs | — |
| `build-python` | maturin develop (release) | setup-python |

### xtask Commands

| Command | Behavior |
|---------|----------|
| `build-plugins` | Iterates plugin dirs with Cargo.toml, builds each in release mode, copies `.so`/`.dylib`/`.dll` + binaries back to plugin dir |
| `build-c-plugins` | Finds single `.c` per plugin dir, compiles with `gcc -shared -fPIC -ljansson`, includes `engine/` in include path |
| `build-wasm-tests` | Compiles `test_*.rs` files in `engine_wasm/wasm_tests/` via `rustc --target wasm32-unknown-unknown --crate-type=cdylib` |
| `build-all` | Chains all 3 above |

### Additional System Dependencies
- **LuaJIT**: `libluajit-5.1-dev` + `pkg-config` + `PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig`
- **C plugins**: `gcc` + `libjansson-dev`
- **WASM**: `rustup target add wasm32-unknown-unknown`
- **Python**: `python3`, `venv`, `maturin`, `pytest`

## 9. Component Schemas

### 25 JSON Schema files in `engine/assets/schemas/`

| Schema | Modes | Notes |
|--------|-------|-------|
| agent.json | colony, roguelike | Agent type/behavior |
| base_stats.json | colony | Base stats for entities |
| body.json | colony, roguelike | Body slots/parts |
| camera.json | colony, roguelike | Camera settings |
| corpse.json | colony, roguelike | Corpse state |
| decay.json | colony, roguelike | Decay timer |
| equipment.json | colony, roguelike | Equipment slots |
| equipment_effects.json | colony | Equipment stat modifiers |
| happiness.json | colony | Happiness metric |
| hazard.json | colony | Environmental hazards |
| health.json | colony, roguelike | HP system |
| inventory.json | colony, roguelike | Item storage |
| item.json | colony, roguelike | Item definition |
| **job.json** (216 lines) | colony, roguelike | Most complex — 13 states, resource requirements, effects |
| map.json | colony | Map metadata |
| position.json | colony, roguelike | Position (Square/Hex/Province) |
| production_job.json | colony | Production job definition |
| region.json | colony | Region definition |
| region_assignment.json | colony | Region assignment |
| renderable.json | colony, roguelike | Visual glyph/color |
| resource.json | colony | Resource definition |
| stats.json | colony | Calculated stats |
| stockpile.json | colony | Stockpile designation |
| terrain.json | colony | Terrain types |
| type.json | colony, roguelike | Entity type |

### Additional Assets
- `engine/assets/jobs/dig_tunnel.json` — one job definition
- `engine/assets/recipes/wood_plank.json` — one recipe
- `engine/assets/resources/wood.json` — one resource

### Schema Features
- Standard JSON Schema with `modes` field (custom extension — not standard JSON Schema)
- Mode field controls which game modes a component is valid in
- Loaded by `engine_core::ecs::schema::load_schemas_from_dir_with_modes()`
- Validated offline by `schema_validator` CLI

## 10. Job System

The job system is the most complex subsystem in the engine, organized into 9 subdirectories:

### Architecture

```
systems/job/
├── ai/              # AI job assignment
│   ├── event_intent.rs          # AI event→intent mapping
│   ├── event_reaction_system.rs # Reaction event handling
│   └── logic.rs                 # Core AI job selection logic
├── board/           # Job board (scheduling + queuing)
│   ├── job_board.rs             # Central job queue
│   └── priority_aging.rs        # Priority aging mechanism
├── core/            # Core job definitions
│   ├── children.rs              # Child job spawning
│   ├── dependencies.rs          # Job dependency resolution
│   └── requirements.rs          # Job requirements (skills, items)
├── ops/             # Job operations
│   ├── movement_ops.rs          # Movement operations for jobs
│   └── resource_ops.rs          # Resource operations for jobs
├── registry/        # Handler registries
│   ├── effect_processor_registry.rs
│   └── job_handler_registry.rs
├── reservation/     # Resource reservation
│   └── resource_reservation.rs  # Locking resources for jobs
├── states/          # FSM for job states
│   ├── helpers.rs               # State helper functions
│   ├── transitions.rs           # State transition logic
│   ├── movement/
│   │   ├── at_site.rs           # "at_site" state handling
│   │   ├── going_to_site.rs     # "going_to_site" state
│   │   └── pending.rs           # "pending" state
│   └── resource/
│       ├── delivering.rs        # Resource delivery state
│       └── fetching.rs          # Resource fetching state
├── system/          # Job system orchestrator
│   ├── orchestrator.rs          # Main job system coordinator
│   ├── process.rs               # Job processing pipeline
│   ├── effects.rs               # Job effect execution
│   └── events.rs                # Job event emission
└── types/           # Job type system
    ├── builtin_handlers.rs      # Built-in job type handlers
    ├── job_type.rs              # Job type definitions
    └── loader.rs                # Job type loader
```

### Job State Machine (from job.json schema)
```
pending → fetching_resources → going_to_site → at_site → in_progress → complete
  ↓           ↓                  ↓              ↓           ↓
  → blocked, waiting_for_resources, paused, interrupted, failed, cancelled
```

### Key Job Features
- FIFO/LIFO scheduling via `created_at` timestamp
- Priority aging (higher priority = picked first over time)
- Resource reservation system (lock resources to jobs)
- Child job spawning (composite jobs)
- Dependency graphs between jobs
- Event-based job lifecycle (event emission + reaction)
- AI job assignment with intent resolution

## 11. UI System

### Architecture

```
presentation/
├── input.rs        # InputEvent enum (KeyPress, Quit) + PresentationInput trait
├── layout.rs       # CellLayout trait + SquareLayout, HexLayout, ProvinceLayout
├── renderer.rs     # PresentationRenderer trait + RenderCommand, RenderColor
└── ui/
    ├── event.rs    # UiEvent (Click, KeyPress)
    ├── factory.rs  # UiFactory (widget registry + constructor pattern) + global UI_FACTORY singleton
    ├── schema_loader.rs  # JSON→UI tree builder
    ├── root.rs     # UiRoot: focus management, keyboard navigation (Tab/Arrows)
    ├── layout/
    │   ├── direction.rs    # Direction enum
    │   ├── grid.rs         # Grid layout
    │   └── linear.rs       # Linear layout (horizontal/vertical)
    └── widget/
        ├── widget_trait.rs # UiWidget trait (10+ methods)
        ├── button.rs       # Clickable button
        ├── checkbox.rs     # Toggle checkbox
        ├── context_menu.rs # Right-click context menu
        ├── dropdown.rs     # Dropdown selector
        ├── dynamic.rs      # Dynamically-created widget
        ├── event_log.rs    # Scrollable event log
        ├── focus_grid.rs   # Grid focus navigation
        ├── label.rs        # Text label
        ├── panel.rs        # Container panel
        └── text_input.rs   # Text input field
```

### 10 Widget Types
Button, Checkbox, ContextMenu, Dropdown, DynamicWidget, EventLogWidget, FocusGrid, Label, Panel, TextInput

### 3 Layout Engines
Direction, Grid, Linear

### Key Patterns
- **Factory pattern**: `UiFactory` with string-keyed constructors, global singleton
- **Widget trait**: `UiWidget` with render, handle_event, focus management, z-order, parent/child tree
- **JSON deserialization**: `schema_loader::load_ui_from_json()` — builds widget tree from JSON
- **Focus navigation**: Arrow keys + Tab with directional scoring algorithm
- **Widget registration**: `register_all_widgets()` called at startup

### Terminal Renderer
- `PresentationRenderer` trait abstracts terminal output
- `RenderCommand` = { glyph: char, color: RenderColor, pos: (x, y) }
- `PresentationSystem<R>` provides `render_world()` and `render_map()` — entity rendering with position type detection (Square/Hex/Province)
- Viewport system clips rendering to visible area

## 12. Cross-Language Bindings

### Lua Bindings (engine_lua) — 30 API modules

| Module | File | Purpose |
|--------|------|---------|
| body | lua_api/body.rs | Body slot management |
| camera | lua_api/camera.rs | Camera control |
| callback_registry | lua_api/callback_registry.rs | Lua callback storage |
| component | lua_api/component.rs | Component CRUD |
| death_decay | lua_api/death_decay.rs | Death/decay processing |
| economic | lua_api/economic.rs | Economic system |
| entity | lua_api/entity.rs | Entity lifecycle |
| equipment | lua_api/equipment.rs | Equipment management |
| event_bus | lua_api/event_bus.rs | Event bus API |
| input | lua_api/input.rs | Input handling |
| inventory | lua_api/inventory.rs | Inventory operations |
| job_ai | lua_api/job_ai.rs | AI job assignment |
| job_board | lua_api/job_board.rs | Job board queries |
| job_cancel | lua_api/job_cancel.rs | Job cancellation |
| job_events | lua_api/job_events.rs | Job event listening |
| job_mutation | lua_api/job_mutation.rs | Job CRUD |
| job_query | lua_api/job_query.rs | Job queries |
| job_system | lua_api/job_system.rs | Job system control |
| map | lua_api/map.rs | Map operations |
| mode | lua_api/mode.rs | Game mode switching |
| movement_ops | lua_api/movement_ops.rs | Movement operations |
| region | lua_api/region.rs | Region management |
| save_load | lua_api/save_load.rs | Save/load game state |
| system | lua_api/system.rs | System registration |
| time_of_day | lua_api/time_of_day.rs | Day/night cycle |
| turn | lua_api/turn.rs | Turn processing |
| ui | lua_api/ui.rs | UI operations |
| world | lua_api/world.rs | World API |
| worldgen | lua_api/worldgen.rs | World generation |

Plus: `register_all_api_functions()` in `lua_api/mod.rs` orchestrates registration. Core engine files: `engine.rs`, `event_bus.rs`, `helpers.rs`, `input.rs`, `lib.rs`, `schemas.rs`.

### Python Bindings (engine_py) — 27 API modules

| Module | File | Purpose |
|--------|------|---------|
| body | python_api/body.rs | Body slot management |
| camera_api | python_api/camera_api.rs | Camera control |
| component | python_api/component.rs | Component CRUD |
| death_decay | python_api/death_decay.rs | Death/decay processing |
| economic | python_api/economic.rs | Economic system |
| entity | python_api/entity.rs | Entity lifecycle |
| equipment | python_api/equipment.rs | Equipment management |
| inventory | python_api/inventory.rs | Inventory operations |
| job_ai | python_api/job_ai.rs | AI job assignment |
| job_api | python_api/job_api.rs | General job API |
| job_board | python_api/job_board.rs | Job board queries |
| job_children | python_api/job_children.rs | Child job management |
| job_dependencies | python_api/job_dependencies.rs | Job dependency graph |
| job_events | python_api/job_events.rs | Job event listening |
| job_production | python_api/job_production.rs | Production job API |
| job_query | python_api/job_query.rs | Job queries |
| job_reservation | python_api/job_reservation.rs | Resource reservation |
| map_api | python_api/map_api.rs | Map operations |
| mode | python_api/mode.rs | Game mode switching |
| movement | python_api/movement.rs | Movement operations |
| region | python_api/region.rs | Region management |
| save_load | python_api/save_load.rs | Save/load game state |
| time_of_day | python_api/time_of_day.rs | Day/night cycle |
| turn | python_api/turn.rs | Turn processing |
| ui | python_api/ui.rs | UI operations |
| world | python_api/world.rs | World API (PyWorld) |

**Python** has extra modules not in Lua: `job_children`, `job_dependencies`, `job_production`, `job_reservation` — suggesting Python bindings are more developed for job management.

Core engine files: `api.rs`, `event_bus.rs`, `job_bridge.rs`, `job_logger.rs`, `lib.rs`, `plugin_init.rs`, `system_bridge.rs`, `worldgen_bridge.rs`.

### WASM Bindings (engine_wasm) — Least developed

```
engine_wasm/
├── src/
│   ├── lib.rs           # WasmEngine struct
│   ├── engine.rs        # WASM engine implementation
│   └── host_api/
│       ├── mod.rs
│       └── entity.rs    # Entity API only
├── tests/
│   ├── mod.rs
│   ├── wasm_engine.rs
│   └── wasm_entity_api.rs
└── wasm_tests/
    ├── test_entity_api.rs    # Source
    └── test_entity_api.wasm  # Compiled binary
```

WASM backend is minimal. Only entity API (spawn/get_component) is exposed to WASM guests. 3 Rust integration tests, 1 WASM test module. Runtime: wasmtime with async support.

### Cross-Language API Surface Comparison

| API Area | Lua (30 modules) | Python (27 modules) | WASM |
|----------|:---:|:---:|:----:|
| Entity lifecycle | ✓ | ✓ | ✓ (partial) |
| Component CRUD | ✓ | ✓ | — |
| Map operations | ✓ | ✓ | — |
| Movement | ✓ | ✓ | — |
| Combat/damage | ✓ | ✓ | — |
| Game mode | ✓ | ✓ | — |
| Job system | 7 modules | 9 modules | — |
| Inventory | ✓ | ✓ | — |
| Equipment | ✓ | ✓ | — |
| Body | ✓ | ✓ | — |
| Economic | ✓ | ✓ | — |
| Save/Load | ✓ | ✓ | — |
| Worldgen | ✓ | ✓ | — |
| Event bus | ✓ | ✓ | — |
| UI | ✓ | ✓ | — |
| Region | ✓ | ✓ | — |
| Camera | ✓ | ✓ | — |
| Input | ✓ | — | — |
| Turn/Time | ✓ | ✓ | — |
| Callback registry | ✓ | — | — |

## 13. Findings

### Key Structural Observations

1. **Schema-driven ECS** is the backbone — all components defined as JSON schemas, loaded dynamically. Rust-side `#[component]` macro generates versioning/migration/serde/schema.

2. **Feature flags are vestigial** — `lua`, `python`, `wasm` features on engine_core are empty placeholders with no `cfg` gates. engine_core always links mlua regardless.

3. **Two plugin systems coexist**: Native ABI (`.so` via libloading + PluginVTable) and subprocess (Unix socket JSON protocol). Only native ABI is actively used.

4. **Job system is the most complex subsystem** — 9 subdirectories, ~40 source files, FSM with 13 states, resource reservation, AI assignment, dependency graphs, priority aging.

5. **Stale nested Cargo.lock** in `engine/core/Cargo.lock` — the root workspace `Cargo.lock` is authoritative.

6. **Codegen is unwired** — `engine/tools/codegen/` exists but is not called by any build step. Generated code would need manual regeneration.

7. **Only 1 TODO comment** found across the entire Rust codebase: `// TODO: implement province centroid` in `presentation/layout.rs:46`.

8. **WASM backend is minimal** compared to Lua and Python — only entity API exposed.

9. **Schema `modes` field is non-standard JSON Schema** — this custom extension has no `$schema` meta-schema definition and is validated by custom logic in `schema_validator`.

10. **engine_core depends on schema_validator as a library** — unusual dependency where a tool crate is also a library dependency of the core engine.

### Changes from Prior Audit

This is a refresh of the same-day prior audit (`analysis-codebase-map-2026-05-27.md`). Key differences/additions:

- **More granular module mapping**: All ~95 engine_core source files listed with descriptions
- **Job system deep-dive**: All 9 subdirectories, their files, and the 13-state FSM documented
- **UI system comprehensive**: All 10 widgets, 3 layouts, factory pattern, focus navigation, schema loader documented
- **Cross-language API surface comparison table**: Which APIs exist in Lua vs Python vs WASM
- **Plugin manifests**: All 5 plugin.json files documented with their exact format
- **System dependencies**: LuaJIT, C, WASM toolchain requirements documented
- **Mod loading flow**: Full pipeline from schema loading to script execution documented
- **CI dependency graph**: Parallel build stages with artifact passing documented
- **Binary entrypoints**: All 6 binaries with file sizes/line counts documented

## 14. Risks & Unknowns

- **Nightly Rust only** — edition 2024 is not stable. CI must pin nightly toolchain.
- **engine_core links LuaJIT unconditionally** — feature flags don't control this, so even Python/WASM builds require LuaJIT dev headers.
- **Plugin ABI has no versioning** — the `PluginVTable` struct has no version field. Incompatible plugin binaries would cause UB.
- **Lua test runner is fragile** — relies on source-parsing comments from `.lua` files (regex-based), not actual Lua execution.
- **No integration test coverage for codegen** — the tool has no tests and no build integration.
- **Python tests depend on maturin** — requires full Rust compilation before testing, making test-python the slowest step.
- **Only 1 mod exists** — the mod system is barely exercised beyond the mvp_roguelike demo.
- **WASM backend is experimental** — only 1 WASM test module, no WASM API surface beyond entity operations.

## 15. Recommendations

1. Remove stale `engine/core/Cargo.lock` to avoid confusion.
2. Either implement feature gates or remove the vestigial feature flags from engine_core.
3. Wire codegen into the build pipeline or remove it.
4. Add ABI version field to `PluginVTable` for safe plugin loading.
5. Expand WASM API surface to match Lua/Python for feature parity.
6. Add more mods to exercise the mod loading system.
7. Implement province centroid calculation (the lone TODO).
8. Define a `$schema` meta-schema for the `modes` extension field.

## References

- Workspace manifest: `/Cargo.toml`
- Build orchestration: `/Makefile`, `/xtask/src/main.rs`
- Game config: `/game.toml`
- Core library: `engine/core/src/lib.rs`
- ECS module: `engine/core/src/ecs/`
- Plugin ABI: `engine/engine_plugin_abi.h`
- Component schemas: `engine/assets/schemas/` (25 files)
- CI workflows: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/lint-schemas.yml`
- Lua API: `engine_lua/src/lua_api/` (30 modules)
- Python API: `engine_py/src/python_api/` (27 modules)
- WASM API: `engine_wasm/src/host_api/` (2 files)
- Job system: `engine/core/src/systems/job/` (9 subdirs)
- UI system: `engine/core/src/presentation/ui/` (10 widgets)
- Test integration tests: `engine/core/tests/` (108 tests)
- Prior analysis: `knowledge/analysis-codebase-map-2026-05-27.md`
- Agent instructions: `AGENTS.md`
