---
title: "ANALYSIS: Codebase Map — MGE"
version: 1.0.0
status: draft
type: analysis
created: 2026-05-27
author: Explorer
superseded_by: null
---

# ANALYSIS: Codebase Map — MGE

## 1. Overview

MGE (Modular Game Engine) is a **cross-language game engine** implemented as a Rust workspace monorepo of **10 crates**. It provides a schema-driven ECS with identical scripting APIs in Lua and Python, hot-reloadable native plugins (Rust/C), WASM sandboxing, and a terminal-based renderer.

### Quick Facts

| Metric | Value |
|--------|-------|
| Total crates | 10 (9 in workspace, 1 standalone stale lockfile) |
| Languages | Rust (edition "2024"), Lua (LuaJIT), Python, C, WASM |
| Rust edition | 2024 (nightly required), except plugins/rust_test_plugin (2021) |
| Build system | Cargo + Makefile + xtask orchestrator |
| CI | GitHub Actions (3 workflows: ci.yml, lint-schemas.yml, release.yml) |
| Release automation | semantic-release via Node.js + @timada/semantic-release-cargo |
| Lines of Rust source | ~15K+ (engine_core alone is substantial) |
| Test files | 108 Rust + 44 Python + 47 Lua + 2 Rust (engine_lua) + 3 Rust (engine_wasm) |

## 2. Workspace Architecture

### Crate Dependency Graph

```
engine_macros <- engine_core <- engine_lua
                               <- engine_py
                               <- engine_wasm
                               (tools: schema_validator, codegen)
                               (xtask — standalone)
                               (rust_test_plugin — standalone)
```

- **engine_core** — Pure Rust core (ECS, map, plugins, systems, presentation, worldgen).
- **engine_lua** — Lua scripting bridge. Depends on engine_core + mlua (LuaJIT). Contains 2 binaries.
- **engine_py** — Python native extension. Depends on engine_core + pyo3. Built as cdylib.
- **engine_wasm** — WASM sandbox. Depends on engine_core + wasmtime.
- **engine_macros** — Proc-macro crate. Standalone, consumed by engine_core. Provides #[component] attribute macro.
- **schema_validator** — Tool at engine/tools/schema_validator/. Validates JSON schemas via CLI.
- **codegen** — Tool at engine/tools/codegen/. Code generation (NOT wired into any build step).
- **xtask** — Build orchestrator. Plugin build/deploy, C plugin compilation, WASM test compilation.
- **rust_test_plugin** — Test plugin (Rust cdylib) at plugins/rust_test_plugin/.

### Editions

| Crate | Edition | Notes |
|-------|---------|-------|
| All except one | 2024 | Requires nightly Rust |
| plugins/rust_test_plugin | 2021 | Only edition 2021 crate |

### Feature Flags on engine_core

```toml
[features]
lua = []
python = []
wasm = []
```

Empty placeholders. No cfg(feature = ...) guards exist in engine_core/src/. The features are vestigial.

### Key Dependencies by Crate

| Crate | Key Dependencies |
|-------|-----------------|
| engine_core | mlua (LuaJIT), serde, schemars, serde_json, bson, semver, thiserror, anyhow, libloading, jsonschema, once_cell, indexmap, topo_sort, serde_yaml, toml, parking_lot, walkdir, lazy_static, dyn-clone |
| engine_macros | syn, quote, proc-macro2, bson |
| engine_lua | engine_core, mlua, serde_json, gag (stdout capture), regex, once_cell |
| engine_py | engine_core, pyo3, serde_json, serde-pyobject, pythonize |
| engine_wasm | engine_core, wasmtime (async), anyhow, serde_json |

## 3. Build Pipeline

### Makefile Targets

| Target | Action | Dependencies |
|--------|--------|-------------|
| all | validate-schema + build-all | -- |
| test / test-all | validate-schema -> test-rust -> test-python -> test-lua | -- |
| validate-schema | schema_validator -- engine/assets/schemas | -- |
| build-plugins | xtask build-plugins | -- |
| build-c-plugins | xtask build-c-plugins | -- |
| build-wasm-tests | xtask build-wasm-tests | -- |
| build-all | chain all 3 xtask builds | -- |
| test-rust | cargo test --all | -- |
| test-python | setup-python -> maturin develop -> pytest | setup-python, build-python |
| test-lua | build mge_lua_test_runner -> run_lua_tests.sh | -- |
| clean | cargo clean | -- |

### xtask Commands

| Command | Behavior |
|---------|----------|
| build-plugins | Iterates plugin dirs with Cargo.toml, builds each in release, copies .so/.dylib/.dll + binaries back |
| build-c-plugins | Finds single .c per plugin dir, compiles with gcc -shared -fPIC -ljansson |
| build-wasm-tests | Compiles test_*.rs files in engine_wasm/wasm_tests/ via rustc --target wasm32-unknown-unknown --crate-type=cdylib |
| build-all | Chains all 3 above |

### CI Pipeline (ci.yml) — Enforced Order

lint -> validate-schema + build-c-plugins + build-wasm-tests -> build-all -> test-rust -> test-lua -> test-python

### Release Pipeline (release.yml)

- Runs npx semantic-release on push to main
- Uses @timada/semantic-release-cargo to publish Cargo crates
- Commit format: <type>(<scope>): <subject> (enforced by .gitmessage)

### Lint Schemas (lint-schemas.yml)

- Path-filtered to schema changes only
- Runs schema_validator with --summary-only

## 4. Source Tree Map

### engine/ Directory

```
engine/
├── assets/
│   ├── schemas/            # 25 component JSON schemas
│   ├── jobs/               # dig_tunnel.json
│   ├── recipes/            # wood_plank.json
│   └── resources/          # wood.json
├── core/
│   ├── Cargo.toml
│   ├── Cargo.lock          <- STALE (root workspace lockfile is authoritative)
│   ├── examples/
│   │   ├── viewport_demo.rs
│   │   └── viewport_camera_demo.rs
│   ├── src/
│   │   ├── lib.rs          # Public API: World, components, modes
│   │   ├── main.rs         # Plugin subprocess handler (Unix socket)
│   │   ├── config.rs       # GameConfig
│   │   ├── ecs/
│   │   │   ├── mod.rs      # Re-exports: World, ComponentRegistry, ComponentSchema
│   │   │   ├── assets.rs, error.rs, event.rs, event_bus_registry.rs, event_logger.rs
│   │   │   ├── registry.rs, schema.rs, system.rs
│   │   │   ├── components/ # camera, corpse, decay, happiness, health, inventory, position
│   │   │   └── world/      # component, entity, events, job_handlers, map, mode, resources, save_load, systems, wasm
│   │   ├── map/            # cell_key, deserialize, hex, pathfinding, province, square, topology
│   │   ├── modes/          # GameMode enum, ModeRestrictedComponent trait
│   │   ├── mods/           # loader, manifest
│   │   ├── plugins/        # ffi, loader, manager, registry, subprocess, types, dynamic_systems
│   │   ├── presentation/   # input, layout, renderer, ui/ (12 widgets, 3 layouts, event system, factory)
│   │   ├── systems/        # death_decay, economic/, inventory, movement, equipment, body_sync, stat_calc, job/ (9 subdirs)
│   │   └── worldgen.rs
│   └── tests/              # 108 integration tests
├── engine_plugin_abi.h     # C ABI header
├── scripts/
│   └── lua/
│       ├── demos/          # hello.lua, roguelike_mvp.lua, stockpile_demo.lua, death_removal_demo.lua
│       └── tests/          # 47 test files, helpers/ (2 files)
└── tools/
    ├── codegen/            # NOT wired into build
    └── schema_validator/   # CLI tool
```

### Plugins Directory

| Plugin | Language | Source | Output |
|--------|----------|--------|--------|
| rust_test_plugin | Rust cdylib (edition 2021) | src/lib.rs + src/main.rs | librust_test_plugin.so |
| simple_square_plugin | C | simple_square_plugin.c | libsimple_square_plugin.so |
| simple_hex_plugin | C | simple_hex_plugin.c | libsimple_hex_plugin.so |
| simple_province_plugin | C | simple_province_plugin.c | libsimple_province_plugin.so |
| test_plugin | C | test_plugin.c | libtest_plugin.so |

### Mods Directory

```
mods/mvp_roguelike/
├── mod.json          # name, version, mode="roguelike", schemas[], systems[], main_script
├── assets/map1.json
├── schemas/          # camera, health, item, monster, player, renderable (6 schemas)
└── systems/main.lua  # Game logic
```

## 5. Entrypoints

### All Binary Targets

| Binary | Crate | Path | Purpose |
|--------|-------|------|---------|
| mge_cli | engine_lua | src/bin/mge_cli.rs | Main game CLI. Runs Lua scripts or mods |
| mge_lua_test_runner | engine_lua | src/bin/mge_lua_test_runner.rs | Lua test harness. 380 lines |
| schema_validator | schema_validator | tools/schema_validator/src/main.rs | Validate all JSON schemas |
| xtask | xtask | src/main.rs | Build orchestrator |
| engine_core | engine_core | src/main.rs | Plugin subprocess handler (Unix socket) |
| rust_test_plugin | rust_test_plugin | src/main.rs | Standalone binary from test plugin |

### Examples

| Example | Path | Purpose |
|---------|------|---------|
| viewport_demo | engine/core/examples/viewport_demo.rs | Terminal viewport rendering demo |
| viewport_camera_demo | engine/core/examples/viewport_camera_demo.rs | Camera + viewport interaction demo |

## 6. Test Infrastructure

### Test Locations & Counts

| Location | Count | Runner | Notes |
|----------|-------|--------|-------|
| engine/core/tests/ | 108 | cargo test | Integration tests |
| engine_py/tests/ | 44 | pytest | Needs maturin develop first |
| engine/scripts/lua/tests/ | 47 + 2 helpers | mge_lua_test_runner | Source-parsing discovery |
| engine_lua/tests/ | 2 | cargo test | Script loading + input tests |
| engine_wasm/tests/ | 3 | cargo test | WASM engine integration |
| engine_macros/tests/ | ? | cargo test | Proc-macro tests |
| schema_validator/tests/ | ? | cargo test | Schema validator tests |

## 7. Cross-Language Architecture

### Lua Bindings (engine_lua)
- 30 modules in engine_lua/src/lua_api/
- Runtime: LuaJIT via mlua crate
- Full API surface: entity, component, map, movement, combat, mode, simulation, jobs, equipment, inventory, body, camera, input, UI, event bus, worldgen, save/load, time-of-day, death/decay, economic

### Python Bindings (engine_py)
- 27 modules in engine_py/src/python_api/
- Same API as Lua + extra modules: job_production, job_children, job_dependencies, job_reservation
- Built as cdylib via maturin

### WASM Bindings (engine_wasm)
- Least developed: only entity API exposed
- Runtime: Wasmtime (async)
- 2 host API files, 3 WASM test files

### C ABI (engine/engine_plugin_abi.h)
- PluginVTable with init, shutdown, update, worldgen, system registration, hot-reload
- EngineApi exposes spawn_entity and set_component

## 8. Schema System

### All 25 Component Schemas
agent, base_stats, body, camera, corpse, decay, equipment, equipment_effects, happiness, hazard, health, inventory, item, job (216 lines, most complex), map, position, production_job, region, region_assignment, renderable, resource, stats, stockpile, terrain, type

### Schema Features
- Standard JSON Schema with modes field (custom extension)
- Loaded by engine_core::ecs::schema::load_schemas_from_dir_with_modes()
- Validated offline by schema_validator

## 9. Key Observations

1. Schema-driven ECS is the backbone
2. Feature flags are vestigial (empty placeholders with no cfg gates)
3. engine_core depends on mlua despite being "pure Rust" — links to LuaJIT always
4. Two plugin systems coexist: native ABI and subprocess (Unix socket)
5. Job system is the most complex subsystem (9 subdirectories)
6. Stale nested Cargo.lock in engine/core/
7. Codegen is unwired — not called by any build step
8. Only 1 TODO comment found across the entire codebase
9. WASM backend is minimal compared to Lua and Python
10. Schema modes field is non-standard JSON Schema extension
