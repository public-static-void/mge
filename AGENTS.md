# MGE — Instructions for AI Agents

## Project Identity

Rust workspace monorepo (9 crates) — cross-language game engine. Languages: Rust (edition "2024"), Lua, Python, C, WASM.

**Rust edition 2024 requires nightly Rust.** 8 of 9 crates use `edition = "2024"`. The only exception is `plugins/rust_test_plugin/` (edition "2021").

Build system: Cargo + Makefile (orchestration) + xtask (plugin deploy).

---

## Quick Start

```sh
make all                    # validate schemas → build everything
make test                   # validate-schema + test-rust + test-python + test-lua
make clean                  # cargo clean
```

---

## Commands

### Build

| What | Command |
|---|---|
| Full build | `make all` |
| Rust plugins | `cargo run -p xtask -- build-plugins` |
| C plugins | `cargo run -p xtask -- build-c-plugins` |
| WASM tests | `cargo run -p xtask -- build-wasm-tests` |
| Python native ext | `make test-python` (builds via maturin inside venv) |
| Schema validation | `cargo run --bin schema_validator --release -- engine/assets/schemas` |

### Run

| What | Command |
|---|---|
| Game CLI (Lua script) | `cargo run --bin mge_cli -- engine/scripts/lua/demos/roguelike_mvp.lua` |
| Game CLI (mod) | `cargo run --bin mge_cli -- --mod mvp_roguelike` |
| Viewport demo | `cargo run --example viewport_demo -p engine_core` |

### Test

| What | Command |
|---|---|
| All tests | `make test` |
| Rust only | `make test-rust` (alias: `cargo test --all`) |
| Single Rust test | `cargo test -p engine_core --test <test_file> <test_name>` |
| Python tests | `cd engine_py && source .venv/bin/activate && pytest tests/ -k <filter>` |
| Lua tests | `./run_lua_tests.sh <module_filter> [function_filter]` |
| Schema validation | `make validate-schema` |

### Lint

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Required Command Order

CI enforces this sequence:

```
validate-schema → build-c-plugins → build-wasm-tests → build-all → test-rust → test-python → test-lua
```

`make test` runs: `validate-schema → test-rust → test-python → test-lua` (sequential). Python requires `maturin develop` (handled by `build-python` target in `make test-python`). Lua tests require C plugin `.so` at `plugins/simple_square_plugin/libsimple_square_plugin.so`.

---

## Monorepo Boundaries

### Crate Dependency Graph

```
engine_macros ← engine_core ← engine_lua
                              ← engine_py
                              ← engine_wasm
```

- `engine_core` has **no language binding dependencies** — pure Rust core.
- `engine_lua` depends on `engine_core` + `mlua` (LuaJIT).
- `engine_py` depends on `engine_core` + `pyo3`.
- `engine_wasm` depends on `engine_core` + `wasmtime`.
- `engine_macros` is standalone (proc-macro), consumed by `engine_core`.

### Entrypoints (binary targets)

| Binary | Crate | Path | Purpose |
|---|---|---|---|
| `mge_cli` | `engine_lua` | `src/bin/mge_cli.rs` | Main game CLI — run Lua scripts or mods |
| `mge_lua_test_runner` | `engine_lua` | `src/bin/mge_lua_test_runner.rs` | Lua integration test harness |
| `schema_validator` | `schema_validator` | `tools/schema_validator/src/main.rs` | Validate all JSON schemas |
| `xtask` | `xtask` | `src/main.rs` | Plugin build/deploy orchestrator |

### Feature flags on `engine_core`

```toml
[features]
lua = []
python = []
wasm = []
```

Currently empty placeholders — control which language bridges are active.

### Plugin Source Directories

```
plugins/
  rust_test_plugin/         # Rust cdylib
  simple_square_plugin/     # C plugin
  simple_hex_plugin/        # C plugin
  simple_province_plugin/   # C plugin
  test_plugin/              # C plugin
```

### Mods (game content packages)

```
mods/mvp_roguelike/
  mod.json          # { name, version, mode, schemas[], systems[], main_script }
  schemas/          # Component schemas
  systems/          # Lua system scripts
```

---

## Critical Gotchas

### Build / Toolchain

1. **Nightly Rust required.** Edition "2024" is not stable. Use `rustup toolchain install nightly && rustup default nightly`.
2. **LuaJIT system dep.** Install `libluajit-5.1-dev` + `pkg-config`. CI sets `PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig`.
3. **C plugins need gcc + libjansson-dev.** xtask finds single `.c` file per plugin dir, compiles to `.so` with `-shared -fPIC -ljansson`.
4. **engine/core/Cargo.lock is stale.** The root workspace `Cargo.lock` is authoritative. Do not use the nested one.

### Environment Variables

- `LD_LIBRARY_PATH` — must include `$PWD/plugins` for native plugin loading (set before running tests).
- `MGE_SCHEMA_DIR` — override schema path from `engine/assets/schemas`.
- `MGE_CONFIG_FILE` — override config from `game.toml`.
- `EXTRA_INCLUDE` — extra include path for C plugin compilation.

### Plugin Deployment

xtask builds each Rust plugin crate in release mode, then copies `target/release/lib<name>.so` into the plugin's own directory. Tests load from `plugins/<name>/lib<name>.so`.

### Python

`make test-python` creates a venv in `engine_py/.venv/`, runs `maturin develop --release`, then `pytest`. The `.so` only exists inside the venv. Re-run after any `engine_core` changes.

### Codegen Tool

`engine/tools/codegen/` exists but is **not wired into any build step**. Generated code may need explicit regeneration.

---

## Testing Quirks

### Rust Tests

- 108 integration tests in `engine/core/tests/`.
- Require pre-built C plugins and WASM test modules to exist.
- CI workflow: build-all → download artifacts → `cargo run -p xtask -- build-plugins` → `export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:$(pwd)/plugins` → `cargo test --all`.

### Lua Tests

- 47 test files in `engine/scripts/lua/tests/`.
- Test discovery is **source-parsing based** — the Rust test runner (`mge_lua_test_runner`) reads each `.lua` file, strips comments, and parses `return { test_xxx = function() ... end }` patterns statically. Does NOT require the Lua module at parse time.
- Each test gets a **fresh World instance** — full state isolation.
- Requires C plugin at `plugins/simple_square_plugin/libsimple_square_plugin.so`.
- Pre-registered systems: `ProcessDeaths`, `ProcessDecay`, `EconomicSystem`, `JobSystem`, `InventoryConstraintSystem`, `EquipmentLogicSystem`, `BodyEquipmentSyncSystem`.
- Test helpers: `engine/scripts/lua/tests/helpers/` (`job_helpers.lua`, `ai_job_helpers.lua`).

### Python Tests

- 44 test files in `engine_py/tests/`.
- Fixture via `conftest.py`: `make_world()` creates `PyWorld(schema_dir)` with job event logger initialized.
- Schema dir is relative: `../../engine/assets/schemas` from `engine_py/tests/`.
- Requires `maturin develop` first (handled by `make test-python`).

---

## Repo Conventions

- **Commit format:** `<type>(<scope>): <subject>` — enforced by `.gitmessage` and used by `semantic-release` via `.releaserc.json`.
- **Schema-driven ECS:** All components defined as JSON schemas in `engine/assets/schemas/` (25 files). Loaded dynamically into `ComponentRegistry`. Rust-side components use `#[component]` macro for auto-generated versioning/migration/serde/schema.
- **Game config:** `game.toml` at workspace root defines title, version, allowed game modes, and native plugin paths.
- **Plugin ABI:** C ABI defined in `engine/engine_plugin_abi.h`. Exports `PluginVTable` with init, shutdown, update, worldgen, system registration, hot-reload.
- **Presentation layer:** Terminal-based renderer with viewport support (terminal roguelike-style output). Demo: `cargo run --example viewport_demo -p engine_core`.

---

## Cross-Language Scripting API

Identical API surface in Lua and Python:

- **Entity:** `spawn_entity()`, `despawn_entity(id)`
- **Components:** `set_component(id, name, data)`, `get_component(id, name)`, `remove_component(id, name)`, `list_components()`, `get_component_schema(name)`
- **Queries:** `get_entities()`, `get_entities_with_component(name)`, `count_entities_with_type(type)`
- **Map:** `add_cell(x,y,z)`, `add_neighbor(from,to)`, `get_all_cells()`, `find_path(start, goal)`, `entities_in_cell(cell)`
- **Movement:** `move_entity(id, dx, dy)`, `move_all(dx, dy)`
- **Combat:** `damage_entity(id, amount)`, `damage_all(amount)`
- **Mode:** `set_mode(mode)`, `get_mode()`, `get_available_modes()`
- **Simulation:** `tick()`, `get_turn()`, `process_deaths()`, `process_decay()`
- **Worldgen:** `register_worldgen_plugin()`, `invoke_worldgen_plugin()`
- **Jobs:** full job system API (board, query, mutation, events, AI assignment)
