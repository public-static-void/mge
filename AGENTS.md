# MGE — Instructions for AI Agents

## Project Identity

Rust workspace monorepo (8 crates) — cross-language game engine. Languages: Rust (edition "2024"), Lua, Python, C, WASM.

**Rust edition 2024 requires nightly Rust.** All 8 crates use `edition = "2024"`.

Build system: Cargo + Makefile (orchestration) + xtask (plugin deploy).

## Workflow

Development is driven by targeting game genres. See [docs/process.md](docs/process.md) for the methodology.
Genre requirement docs: [docs/genres/colony-sim.md](docs/genres/colony-sim.md), [docs/genres/survival.md](docs/genres/survival.md), [docs/genres/grand-strategy.md](docs/genres/grand-strategy.md), [docs/genres/4x.md](docs/genres/4x.md).
Project roadmap: [docs/ROADMAP.md](docs/ROADMAP.md).

---

## Pre-Commit Gates

Before every commit, run all three gates. Only commit when all three pass.

```sh
# 1. Lint — zero warnings
make lint

# 2. Format — clean output (apply with `make fmt`)
make fmt-check

# 3. Test — all suites green
make test-rust                                             # Rust (per-crate shards)
make test-python                                          # Python
make test-lua                                              # Lua (requires C plugin .so)
make test-wasm                                            # WASM
```

When a gate fails, fix the issue and re-run the gates. Only commit code that lints, formats, and tests cleanly.

---

## Quick Start

```sh
make all                    # validate schemas → build everything
make test                   # validate-schema + test-rust + test-python + test-lua + test-wasm
make clean                  # remove build artifacts
```

---

## Commands

### Build

Run the `make` targets below as single commands — each wraps its full underlying toolchain invocation.

| What | Command |
|---|---|
| Full build | `make all` |
| Rust plugins | `make build-plugins` |
| C plugins | `make build-c-plugins` |
| WASM tests | `make build-wasm-tests` |
| Python native ext | `make test-python` (builds via maturin inside venv) |
| Schema validation | `make validate-schema` |

### Run

| What | Command |
|---|---|
| Game CLI (Lua script) | `make run-cli ARGS="engine/scripts/lua/demos/roguelike_mvp.lua"` |
| Game CLI (mod) | `make run-cli ARGS="--mod mvp_roguelike"` |
| Viewport demo | `make run-demo` |

### Test

| What | Command |
|---|---|
| All tests | `make test` |
| Rust only | `make test-rust` (per-crate shards; see the `test-rust` recipe in `Makefile`) |
| Python tests | `make test-python` |
| Lua tests | `make test-lua LUA_FILTER=<module_filter> [<function_filter>]` (exact names: module = `test_*.lua` stem, e.g. `test_loot`, not `loot`; function = exact key; function filter without module filter unsupported) |
| WASM tests | `make test-wasm` |
| Schema validation | `make validate-schema` |
| Single Rust test | narrow to the owning crate's shard via the `test-rust` recipe in `Makefile` |

### Lint

```sh
make fmt-check
make lint
```

---

## Required Command Order

CI jobs (see `.github/workflows/ci.yml` — Branch B: no `test-wasm` job exists):

```
fmt → clippy (clippy needs fmt)
validate-schema, build-c-plugins, build-wasm-tests (no dependencies)
test-rust (needs validate-schema, build-c-plugins, build-wasm-tests)
test-lua (needs build-c-plugins)
test-python (needs build-c-plugins)
release (needs fmt, clippy, validate-schema, build-c-plugins, build-wasm-tests, test-rust, test-lua, test-python)
```

WASM guest modules are built by the `build-wasm-tests` job and consumed as artifacts (there is no standalone `test-wasm` CI job). CI machines may invoke the raw toolchain directly; agent/user-facing instructions use only `make` targets.

`make test` runs: `validate-schema → test-rust → test-python → test-lua → test-wasm` (sequential). Python requires `maturin develop` (handled by `build-python` target in `make test-python`). Lua tests require C plugin `.so` at `plugins/simple_square_plugin/libsimple_square_plugin.so`.

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
4. **Lua CLI sandbox.** The `mge_cli` VM blocks `os`, `io`, `package`, `debug` stdlibs — `require()`, `dofile()`, `loadfile()` do not exist there. Expose Rust functionality via global functions in `engine_lua/src/lua_api/`. Don't design Lua modules that rely on `require()`. Exception: the Lua test runner (`mge_lua_test_runner`, reached via `make test-lua`) installs a `require` shim that loads helper modules from `engine/scripts/lua/tests/` — `require` in Lua tests is legal; it stays unavailable in `mge_cli` and mods.

### Environment Variables

- `LD_LIBRARY_PATH` — must include `$PWD/plugins` for native plugin loading (set before running tests).
- `MGE_SCHEMA_DIR` — override schema path from `engine/assets/schemas`.
- `MGE_CONFIG_FILE` — override config from `game.toml`.
- `EXTRA_INCLUDE` — extra include path for C plugin compilation.

### Plugin Deployment

xtask builds each Rust plugin crate in release mode, then copies `target/release/lib<name>.so` into the plugin's own directory. Tests load from `plugins/<name>/lib<name>.so`.

### Python

`make test-python` creates a venv in `engine_py/.venv/`, runs `maturin develop --release`, then `pytest`. The `.so` only exists inside the venv. Re-run after any `engine_core` changes.

---

## Testing Quirks

### Rust Tests

- Integration tests in `engine/core/tests/`.
- Require pre-built C plugins and WASM test modules to exist.
- CI workflow (`test-rust` job): download C-plugin and WASM artifacts → `make build-all` → `make test-rust` with `LD_LIBRARY_PATH` including `$(pwd)/plugins`.

### Lua Tests

- Test files in `engine/scripts/lua/tests/`.
- Test discovery is **source-parsing based** — the Rust test runner (`mge_lua_test_runner`) reads each `.lua` file, strips comments, and parses `return { test_xxx = function() ... end }` patterns statically. Does NOT require the Lua module at parse time.
- Each test gets a **fresh World instance** — full state isolation.
- Requires C plugin at `plugins/simple_square_plugin/libsimple_square_plugin.so`.
- Pre-registered systems: `ProcessDeaths`, `ProcessDecay`, `EconomicSystem`, `JobSystem`, `InventoryConstraintSystem`, `EquipmentLogicSystem`, `BodyEquipmentSyncSystem`.
- Test helpers: `engine/scripts/lua/tests/helpers/` (`job_helpers.lua`, `ai_job_helpers.lua`).

### Python Tests

- Test files in `engine_py/tests/`.
- Fixture via `conftest.py`: `make_world()` creates `PyWorld(schema_dir)` with job event logger initialized.
- Schema dir is relative: `../../engine/assets/schemas` from `engine_py/tests/`.
- Requires `maturin develop` first (handled by `make test-python`).

### WASM Tests

- Build guest modules via `make build-wasm-tests`.
- Guest binaries (`engine_wasm/wasm_tests/*.wasm`) are gitignored (`.gitignore`) build outputs — rebuild before testing; green means the `.wasm` files are present on disk plus `make test-wasm` passes.
- Requires `wasmtime` (managed via Cargo, no system dependency).
- Test modules in `engine_wasm/tests/`.
- Loaded into the WASM runtime and executed with full state isolation.

---

## Repo Conventions

- **Commit format:** `<type>(<scope>): <subject>` — enforced by `.gitmessage` and used by `semantic-release` via `.releaserc.json`.
- **Roadmap tracking:** When implementing an item listed in [docs/ROADMAP.md](docs/ROADMAP.md), mark it completed (`[x]`) in the ROADMAP file as part of the implementation.
- **Schema-driven ECS:** All components defined as JSON schemas in `engine/assets/schemas/`. Loaded dynamically into `ComponentRegistry`. Rust-side components use `#[component]` macro for auto-generated versioning/migration/serde/schema.
- **Game config:** `game.toml` at workspace root defines title, version, allowed game modes, and native plugin paths.
- **Plugin ABI:** C ABI defined in `engine/engine_plugin_abi.h`. Exports `PluginVTable` with init, shutdown, update, worldgen, system registration, hot-reload.
- **Presentation layer:** Terminal-based renderer with viewport support (terminal roguelike-style output). Demo: `make run-demo`.
- **Roadmap tracking:** After implementing any ROADMAP item (in `docs/ROADMAP.md`), mark it as `[x]` completed in that file as part of the commit.
- **Lint, format, and test before committing:** See [Pre-Commit Gates](#pre-commit-gates) above. Run all three gates (clippy, fmt, test suites) before committing.

---

## Cross-Language Scripting API

Identical API surface in Lua, Python, and WASM:

- **Entity:** `spawn_entity()`, `despawn_entity(id)`
- **Components:** `set_component(id, name, data)`, `get_component(id, name)`, `remove_component(id, name)`, `list_components()`, `get_component_schema(name)`
- **Queries:** `get_entities()`, `get_entities_with_component(name)`, `count_entities_with_type(type)`
- **Map:** `add_cell(x,y,z)`, `add_neighbor(from,to)`, `get_all_cells()`, `find_path(start, goal)`, `entities_in_cell(cell)`
- **Multi-Scale Maps:** `register_map(name, map_json)`, `set_active_map(name)`, `get_map_names()`, `get_active_map_name()`, `link_maps(source_map, source_cell, target_map, target_cell)`, `enter_map(name, entry_cell)`, `exit_map()`, `map_cell(source_map, source_cell)`, `unmap_cell(target_map, target_cell)` — topology (square/hex/province) and map type (overmap/field/strategic/tactical) are orthogonal; `"overmap"` is a convention, never a reserved topology
- **Movement:** `move_entity(id, dx, dy)`, `move_entity_3d(id, dx, dy, dz)`
- **Camera:** `set_camera(x, y, z?)`, `get_camera() → {x, y, z}` — identical in all three bridges (Lua `z?`, Python `z=0`, WASM guest passes z explicitly; WASM JSON string is the bridge transport, not a shape difference)
- **Combat:** `damage_entity(id, amount)`
- **Mode:** `set_mode(mode)`, `get_mode()`, `get_available_modes()`
- **Noise:** `emit_noise(entity_id, intensity, radius)`, `get_noise_at(x, y, z)`, `set_hearing(entity_id, range, sensitivity?)`, `get_hearing(entity_id)` — NoiseSystem propagates sound via BFS with linear falloff (opaque cells block); Hearing-based AI detection escalates `alert_level` (idle → investigate → chase); Stealth `noise_modifier` reduces emission
- **Simulation:** `tick()`, `get_turn()`, `process_deaths()`, `process_decay()`
- **Worldgen:** `register_worldgen_plugin()`, `invoke_worldgen_plugin()`
- **Jobs:** full job system API (board, query, mutation, events, AI assignment)

---

## Testing Conventions

### Test Placement — Integration Tests in `tests/` Directories

Place all tests in dedicated test files outside production source code:

| Test Type | Location | Example |
|---|---|---|
| Core crate unit/integration tests | `engine/core/tests/test_<module>.rs` | `engine/core/tests/test_faction_reputation.rs` |
| Lua tests | `engine/scripts/lua/tests/test_<feature>.lua` | `engine/scripts/lua/tests/test_faction_reputation.lua` |
| Python tests | `engine_py/tests/test_<feature>.py` | `engine_py/tests/test_faction_reputation.py` |
| WASM tests | `engine_wasm/tests/<feature>.rs` | `engine_wasm/tests/wasm_faction_api.rs` |

### Reasoning

- Source files contain only production code — tests are separate artifacts
- Integration tests in `tests/` directories test the public API, which is the correct boundary
- This prevents source file bloat and keeps the module's public interface readable
- CI runs all test files automatically — no special registration needed

### Test Naming

- Test files: `test_<feature>.rs` (Rust), `test_<feature>.lua` (Lua), `test_<feature>.py` (Python)
- Test functions: `test_<descriptive_name>` (snake_case for all languages)
